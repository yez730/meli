use axum::{http::Method, Json, extract::{Query, Path}};
use axum_sessions_auth::{Auth, Rights, AuthSession};
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    schema::*,
    models::{Account, Merchant, Consumer, NewUser, NewConsumer, NewLoginInfo, NewPasswordLoginProvider, Permission, Role}, authorization_policy, my_date_format
};
use diesel::{
    prelude::*, // for .filter
    data_types::Cents, pg::Pg
}; 

use crate::{models::User, axum_pg_pool::AxumPgPool, utils::{get_connection}, login_managers::{get_login_info, password_login::verify_password}};

use super::{PaginatedListRequest,PaginatedListResponse};

#[derive(Deserialize)]
pub struct LoginRequest{
    //登录名
    pub username:String,
    //密码
    pub password:String,
}

#[derive(Serialize)]
pub struct LoginResponse{
    pub account_id:Uuid,
    pub cellphone:String,
    pub real_name:String,
    pub permissions:Vec<Permission>,
    pub roles:Vec<Role>,
    pub merchant_id:Uuid,
    pub merchant_name:String,
}

pub async fn login_by_username(auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>,Json(req):Json<LoginRequest>)->Result<Json<LoginResponse>,String>{
    let login_info=get_login_info(req.username).map_err(|e|e.to_string())?;
    
    verify_password(login_info.user_id, req.password).map_err(|e|e.to_string())?;
    
    let user=users::dsl::users
            .filter(users::dsl::user_id.eq(login_info.user_id))
            .filter(users::dsl::enabled.eq(true))
            .get_result::<User>(&mut get_connection()).map_err(|e|e.to_string())?;

    let account=accounts::dsl::accounts
        .filter(accounts::dsl::user_id.eq(login_info.user_id))
        .filter(accounts::dsl::enabled.eq(true))
        .get_result::<Account>(&mut get_connection()).map_err(|e|e.to_string())?;

    let merchant=merchants::dsl::merchants
        .filter(merchants::dsl::merchant_id.eq(account.merchant_id))
        .filter(merchants::dsl::enabled.eq(true))
        .get_result::<Merchant>(&mut get_connection())
        .map_err(|e|e.to_string())?;
    
    let permissions=permissions::dsl::permissions
        .filter(permissions::dsl::permission_id.eq_any(serde_json::from_str::<Vec<Uuid>>(&user.permissions).unwrap())) 
        .filter(permissions::dsl::enabled.eq(true))
        .get_results::<Permission>(&mut get_connection())
        .map_err(|e|e.to_string())?;
    let roles=roles::dsl::roles
        .filter(roles::dsl::role_id.eq_any(serde_json::from_str::<Vec<Uuid>>(&user.roles).unwrap())) 
        .filter(roles::dsl::enabled.eq(true))
        .get_results::<Role>(&mut get_connection())
        .map_err(|e|e.to_string())?;

    // set user to cookie
    auth.login_user(user.user_id).await;
    auth.remember_user(true).await;

    let login_response=LoginResponse{
        account_id:account.account_id,
        cellphone:account.cellphone,
        real_name:account.real_name.unwrap_or("no setting".to_string()),
        permissions,
        roles,
        merchant_id:merchant.merchant_id,
        merchant_name:merchant.merchant_name,
    };

    Ok(Json(login_response))
}

pub async fn logout(auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>){
    if let Some(_)= auth.current_user {
        auth.logout_user().await;
    }
}

#[derive(Deserialize)]
pub struct ConsumerRequest{
    pub cellphone:String,
    pub real_name:Option<String>,
    pub gender:Option<String>,
    pub birth_day:Option<NaiveDate>,
}

#[derive(Serialize)]
pub struct ConsumerResponse{
    pub user_id: Uuid,
    pub consumer_id: Uuid,
    pub cellphone:String,
    pub real_name:String,//StringOption<String>,
    pub gender:String,//Option<String>,
    pub birth_day:String,//Option<NaiveDate>,
    pub balance:String,//Option<Cents>, // Cents has no [Serialize]
    pub create_time: String,//chrono::DateTime<Local>,
    pub update_time: String,//chrono::DateTime<Local>,
}

pub async fn get_consumers(Query(params):Query<PaginatedListRequest>, method: Method, auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>)->Result<Json<PaginatedListResponse<ConsumerResponse>>,String>{
    //检查登录
    let cur_user=auth.current_user.ok_or("no login.".to_string())?;
    
    //检查权限
    Auth::<User, Uuid, AxumPgPool>::build([Method::GET], false)
        .requires(Rights::any([
            Rights::permission(authorization_policy::ACCOUNT)
        ]))
        .validate(&cur_user, &method, None)
        .await
        .then_some(())
        .ok_or("no permission.".to_string())?;
    
    let get_consumers_query=|p:&PaginatedListRequest|{
        let mut query=consumers::dsl::consumers.into_boxed();
        if let Some(key)=p.key.as_ref(){
            query=query
                .or_filter(consumers::dsl::cellphone.ilike(format!("{key}%"))) 
                .or_filter(consumers::dsl::real_name.ilike(format!("{key}%")))
        }
        query
    };

    let count=get_consumers_query(&params).count().get_result(&mut get_connection()).map_err(|e|e.to_string())?;
    let data=get_consumers_query(&params)
        .order(consumers::dsl::create_time.desc())
        .limit(params.page_size)
        .offset((params.page_index-1)*params.page_size)
        .get_results::<Consumer>(&mut get_connection())
        .map(|v|v.iter().map(|c|ConsumerResponse{
            user_id:c.user_id,
            consumer_id:c.consumer_id,
            cellphone:c.cellphone.clone(),
            real_name:c.real_name.as_ref().map(|n|n.clone()).unwrap_or("".into()),
            gender:c.gender.as_ref().map(|g|g.clone()).unwrap_or("".into()),
            birth_day:c.birth_day.as_ref().map(|b|format!("{:?}",b)).unwrap_or("".into()),//NaiveDate // b.to_string()
            balance:c.balance.as_ref().map(|c|format!("{:?}",c)).unwrap_or("".into()),//Cents // Cents has no [Serialize]
            create_time:format!("{:?}",c.create_time.format("%Y-%m-%d %H:%M:%S")),
            update_time:format!("{:?}",c.update_time.format("%Y-%m-%d %H:%M:%S")),
            
        }).collect::<Vec<_>>())
        .map_err(|e|e.to_string())?;
    
    Ok(Json(PaginatedListResponse{
        page_index:params.page_index,
        page_size:params.page_size,
        total_count:count,
        data:data,
    }))
}

pub async fn add_consumer(method: Method, auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>,Json(req): Json<ConsumerRequest>)->Result<(),String>{
    //检查登录
    let cur_user=auth.current_user.ok_or("no login.".to_string())?;

    //检查权限
    Auth::<User, Uuid, AxumPgPool>::build([Method::POST], false)
        .requires(Rights::any([
            Rights::permission(authorization_policy::ACCOUNT)
        ]))
        .validate(&cur_user, &method, None)
        .await
        .then_some(())
        .ok_or("no permission.".to_string())?;

    //添加 TODO insert data with enabled settting false, finally set to true.
    // 1. add user
    let user_id=Uuid::new_v4();
    let new_user=NewUser{
        user_id: &user_id,
        description: "test user",
        permissions:&serde_json::to_string(authorization_policy::DEFAULT_PERMISSIONS_OF_CONSUMER).unwrap(),
        roles:"[]",
        enabled:true,
        create_time: Local::now(),
        update_time: Local::now(),
        data: None,
    };
    diesel::insert_into(users::table)
    .values(&new_user)
    .execute(&mut get_connection()).map_err(|e|{
        tracing::error!("{}",e.to_string());
        e.to_string()
    })?;
    
    // 2. add login info / login info provider  //TODO cellphone login info provider
    // 2.1
    let login_info=NewLoginInfo{
        login_info_id: &Uuid::new_v4(),
        login_info_account: &req.cellphone,
        login_info_type: "Username", //TODO get enum variant value string
        user_id: &user_id,
        enabled: true,
        create_time: Local::now(),
        update_time: Local::now(),
    };
    diesel::insert_into(login_infos::table)
    .values(&login_info)
    .execute(&mut get_connection()).map_err(|e|{
        tracing::error!("{}",e.to_string());
        e.to_string()
    })?;
    // 2.2
    let password = b"123456";
    let salt = b"randomsalt";
    let config = argon2::Config::default();
    let hash = argon2::hash_encoded(password, salt, &config).unwrap();
    let new_password_login_provider=NewPasswordLoginProvider{
        user_id: &user_id,
        password_hash: &hash,
        enabled:true,
        create_time: Local::now(),
        update_time: Local::now(),
        data:None
    };
    diesel::insert_into(password_login_providers::table)
    .values(&new_password_login_provider)
    .execute(&mut get_connection()).map_err(|e|{
        tracing::error!("{}",e.to_string());
        e.to_string()
    })?;

    // 3. add consumer.
    let new_consumer=NewConsumer{
        user_id:  &user_id,
        consumer_id: &Uuid::new_v4(),
        cellphone:&req.cellphone,
        real_name:req.real_name.as_deref(),
        gender:req.gender.as_deref(),
        birth_day:req.birth_day,
        balance:None,
        enabled:true,
        create_time: Local::now(),
        update_time: Local::now(),
        data: None,
    };
    diesel::insert_into(consumers::table)
    .values(&new_consumer)
    .execute(&mut get_connection()).map_err(|e|{
        tracing::error!("{}",e.to_string());
        e.to_string()
    })?;

    Ok(())
}

pub async fn delete_consumer(Path(id):Path<Uuid>, method: Method, auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>)->Result<(),String>{
    //检查登录
    let cur_user=auth.current_user.ok_or("no login.".to_string())?;

    //检查权限
    Auth::<User, Uuid, AxumPgPool>::build([Method::DELETE], false)
        .requires(Rights::any([
            Rights::permission(authorization_policy::ACCOUNT)
        ]))
        .validate(&cur_user, &method, None)
        .await
        .then_some(())
        .ok_or("no permission.".to_string())?;
    
    // 先不实际删除数据
    diesel::update(
        consumers::dsl::consumers
        .filter(consumers::dsl::consumer_id.eq(id))
        .filter(consumers::dsl::enabled.eq(true))
    )
    .set((
            consumers::dsl::enabled.eq(false),
            consumers::dsl::update_time.eq(Local::now())
        ))
    .execute(&mut get_connection()).map_err(|e|{
        tracing::error!("{}",e.to_string());
        e.to_string()
    })?;

    Ok(())
}

pub async fn update_consumer(Path(id):Path<Uuid>, method: Method, auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>,Json(req): Json<ConsumerRequest>)->Result<(),String>{
    //检查登录
    let cur_user=auth.current_user.ok_or("no login.".to_string())?;

    //检查权限
    Auth::<User, Uuid, AxumPgPool>::build([Method::POST], false)
        .requires(Rights::any([
            Rights::permission(authorization_policy::ACCOUNT)
        ]))
        .validate(&cur_user, &method, None)
        .await
        .then_some(())
        .ok_or("no permission.".to_string())?;
    
    diesel::update(
        consumers::dsl::consumers
        .filter(consumers::dsl::consumer_id.eq(id))
        .filter(consumers::dsl::enabled.eq(true))
    )
    .set((
            consumers::dsl::cellphone.eq(req.cellphone),
            consumers::dsl::real_name.eq(req.real_name),
            consumers::dsl::gender.eq(req.gender),
            consumers::dsl::birth_day.eq(req.birth_day),
            consumers::dsl::update_time.eq(Local::now())
        ))
    .execute(&mut get_connection()).map_err(|e|{
        tracing::error!("{}",e.to_string());
        e.to_string()
    })?;
    
    Ok(())
}
