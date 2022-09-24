use axum::{http::{Method, StatusCode}, Json, extract::{Query, Path}};
use axum_database_sessions::AxumSessionStore;
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
pub struct Identity{
    pub user:UserResponse,
    pub account:Option<AccountResonse>,
    pub consumer:Option<Consumer>,
}

#[derive(Serialize)]
pub struct UserResponse{
    #[serde(flatten)]
    pub user:User,
    pub permissions:Vec<Permission>,
    pub roles:Vec<Role>,
}

#[derive(Serialize)]
pub struct AccountResonse{
    #[serde(flatten)]
    account :Account,

    merchant:Merchant
}

pub async fn login_by_username(auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>,Json(req):Json<LoginRequest>)->Result<Json<Identity>,(StatusCode,String)>{
    let login_info=get_login_info(req.username).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    
    verify_password(login_info.user_id, req.password).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    
    let user=users::dsl::users
            .filter(users::dsl::user_id.eq(login_info.user_id))
            .filter(users::dsl::enabled.eq(true))
            .get_result::<User>(&mut get_connection()).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;

    let account=accounts::table
        .inner_join(merchants::table.on(accounts::merchant_id.eq(merchants::merchant_id)))
        .filter(accounts::dsl::user_id.eq(user.user_id))
        .filter(accounts::dsl::enabled.eq(true))
        .get_result::<(Account,Merchant)>(&mut get_connection())
        .ok() //TODO track error
        .map(|a_m| AccountResonse{
            account:a_m.0,
            merchant:a_m.1,
        });
    let consumer=consumers::dsl::consumers
        .filter(consumers::dsl::user_id.eq(user.user_id))
        .filter(consumers::dsl::enabled.eq(true))
        .get_result::<Consumer>(&mut get_connection())
        .ok();
    
    let permissions=permissions::dsl::permissions
        .filter(permissions::dsl::permission_id.eq_any(serde_json::from_str::<Vec<Uuid>>(&user.permissions).unwrap())) 
        .filter(permissions::dsl::enabled.eq(true))
        .get_results::<Permission>(&mut get_connection())
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    let roles=roles::dsl::roles
        .filter(roles::dsl::role_id.eq_any(serde_json::from_str::<Vec<Uuid>>(&user.roles).unwrap())) 
        .filter(roles::dsl::enabled.eq(true))
        .get_results::<Role>(&mut get_connection())
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    let user_response=UserResponse {user,permissions,roles};

    // set user to cookie
    auth.login_user(user_response.user.user_id).await;
    auth.remember_user(true).await;

    let identity=Identity{
        user:user_response,
        account:account,
        consumer:consumer,
    };

    Ok(Json(identity))
}

pub async fn logout(auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>){
    if let Some(_)= auth.current_user {
        auth.logout_user().await;
    }
}

pub async fn get_current_identity(auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>)->Result<Json<Identity>,(StatusCode,String)>{
    let user=auth.current_user.ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;
   
    let account=accounts::table
        .inner_join(merchants::table.on(accounts::merchant_id.eq(merchants::merchant_id)))
        .filter(accounts::dsl::user_id.eq(user.user_id))
        .filter(accounts::dsl::enabled.eq(true))
        .get_result::<(Account,Merchant)>(&mut get_connection())
        .ok() //TODO track error
        .map(|a_m| AccountResonse{
            account:a_m.0,
            merchant:a_m.1,
        });
    let consumer=consumers::dsl::consumers
        .filter(consumers::dsl::user_id.eq(user.user_id))
        .filter(consumers::dsl::enabled.eq(true))
        .get_result::<Consumer>(&mut get_connection())
        .ok();

    let permissions=permissions::dsl::permissions
        .filter(permissions::dsl::permission_id.eq_any(serde_json::from_str::<Vec<Uuid>>(&user.permissions).unwrap())) 
        .filter(permissions::dsl::enabled.eq(true))
        .get_results::<Permission>(&mut get_connection())
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    let roles=roles::dsl::roles
        .filter(roles::dsl::role_id.eq_any(serde_json::from_str::<Vec<Uuid>>(&user.roles).unwrap())) 
        .filter(roles::dsl::enabled.eq(true))
        .get_results::<Role>(&mut get_connection())
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    let user_response=UserResponse {user,permissions,roles};

    let identity=Identity{
        user:user_response,
        account:account,
        consumer:consumer,
    };

    Ok(Json(identity))
}


#[derive(Deserialize)]
pub struct ConsumerRequest{
    pub cellphone:String,
    pub real_name:Option<String>,
    pub gender:Option<String>,
    pub birth_day:Option<NaiveDate>,
}

pub async fn get_consumers(
    Query(params):Query<PaginatedListRequest>, 
    method: Method, 
    auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>,
    store: AxumSessionStore<AxumPgPool>
)->Result<Json<PaginatedListResponse<Consumer>>,(StatusCode,String)>{
    //检查登录
    let cur_user=auth.current_user.ok_or((StatusCode::INTERNAL_SERVER_ERROR,"no login".to_string()))?;
    
    //检查权限
    Auth::<User, Uuid, AxumPgPool>::build([Method::GET], false)
        .requires(Rights::any([
            Rights::permission(authorization_policy::ACCOUNT)
        ]))
        .validate(&cur_user, &method, store.client.as_ref())
        .await
        .then_some(())
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let get_consumers_query=|p:&PaginatedListRequest|{
        let mut query=consumers::dsl::consumers
            .filter(consumers::dsl::enabled.eq(true))
            .into_boxed();
        if let Some(key)=p.key.as_ref(){
            query=query
                .or_filter(consumers::dsl::cellphone.ilike(format!("{key}%"))) 
                .or_filter(consumers::dsl::real_name.ilike(format!("{key}%")))
        }
        query
    };

    let count=get_consumers_query(&params).count().get_result(&mut get_connection()).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    let data=get_consumers_query(&params)
        .order(consumers::dsl::create_time.desc())
        .limit(params.page_size)
        .offset(params.page_index*params.page_size)
        .get_results::<Consumer>(&mut get_connection())
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    
    Ok(Json(PaginatedListResponse{
        page_index:params.page_index,
        page_size:params.page_size,
        total_count:count,
        data:data,
    }))
}

pub async fn add_consumer(
    method: Method, 
    auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>,
    store: AxumSessionStore<AxumPgPool>,
    Json(req): Json<ConsumerRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录
    let cur_user=auth.current_user.ok_or((StatusCode::INTERNAL_SERVER_ERROR,"no login".to_string()))?;

    //检查权限
    Auth::<User, Uuid, AxumPgPool>::build([Method::POST], false)
        .requires(Rights::any([
            Rights::permission(authorization_policy::ACCOUNT)
        ]))
        .validate(&cur_user, &method, store.client.as_ref())
        .await
        .then_some(())
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR,"no permission.".to_string()))?;

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
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
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
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
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
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

    // 3. add consumer.
    let new_consumer=NewConsumer{
        user_id:  &user_id,
        consumer_id: &Uuid::new_v4(),
        cellphone:&req.cellphone,
        real_name:req.real_name.as_deref(),
        gender:req.gender.as_deref(),
        birth_day:req.birth_day,
        balance:Cents(0),
        enabled:true,
        create_time: Local::now(),
        update_time: Local::now(),
        data: None,
    };
    diesel::insert_into(consumers::table)
    .values(&new_consumer)
    .execute(&mut get_connection()).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

    Ok(())
}

pub async fn delete_consumer(
    Path(id):Path<Uuid>, 
    method: Method, 
    auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>,
    store: AxumSessionStore<AxumPgPool>,
)->Result<(),(StatusCode,String)>{
    //检查登录
    let cur_user=auth.current_user.ok_or((StatusCode::INTERNAL_SERVER_ERROR,"no login".to_string()))?;

    //检查权限
    Auth::<User, Uuid, AxumPgPool>::build([Method::DELETE], false)
        .requires(Rights::any([
            Rights::permission(authorization_policy::ACCOUNT)
        ]))
        .validate(&cur_user, &method, store.client.as_ref())
        .await
        .then_some(())
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    // 先不实际删除数据
    let count=diesel::update(
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
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

    if count!=1 {
        return Err((StatusCode::NOT_FOUND,"data not exists".to_string()));
    }

    Ok(())
}

pub async fn update_consumer(
    Path(id):Path<Uuid>, 
    method: Method, 
    auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>,
    store: AxumSessionStore<AxumPgPool>,
    Json(req): Json<ConsumerRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录
    let cur_user=auth.current_user.ok_or((StatusCode::INTERNAL_SERVER_ERROR,"no login".to_string()))?;

    //检查权限
    Auth::<User, Uuid, AxumPgPool>::build([Method::POST], false)
        .requires(Rights::any([
            Rights::permission(authorization_policy::ACCOUNT)
        ]))
        .validate(&cur_user, &method, store.client.as_ref())
        .await
        .then_some(())
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
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
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;
    
    Ok(())
}

pub async fn get_consumer(
    Path(id):Path<Uuid>, 
    method: Method, 
    auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>,
    store: AxumSessionStore<AxumPgPool>,
)->Result<Json<Consumer>,(StatusCode,String)>{
    //检查登录
    let cur_user=auth.current_user.ok_or((StatusCode::INTERNAL_SERVER_ERROR,"no login".to_string()))?;

    //检查权限
    Auth::<User, Uuid, AxumPgPool>::build([Method::GET], false)
        .requires(Rights::any([
            Rights::permission(authorization_policy::ACCOUNT)
        ]))
        .validate(&cur_user, &method, store.client.as_ref())
        .await
        .then_some(())
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let consumer=consumers::dsl::consumers
        .filter(consumers::dsl::consumer_id.eq(id))
        .filter(consumers::dsl::enabled.eq(true))
        .get_result::<Consumer>(&mut get_connection())
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    
    Ok(Json(consumer))
}
