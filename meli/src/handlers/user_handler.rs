use axum::{http::StatusCode, Json, extract::{Query, Path, State}};
use axum_session_authentication_middleware::session::AuthSession;
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    schema::*,
    models::{Account, Merchant, Consumer, NewUser, NewConsumer, NewLoginInfo, NewPasswordLoginProvider}, authorization_policy
};
use diesel::{
    prelude::*, // for .filter
    data_types::Cents,
}; 

use crate::{models::User, axum_pg_pool::AxumPgPool, login_managers::{get_login_info, password_login::verify_password}};

use super::{PaginatedListRequest,PaginatedListResponse};
use axum_session_authentication_middleware::{ user as auth_user,session::Authentication};

#[derive(Deserialize)]
pub struct LoginRequest{
    pub username:String,
    pub password:String,
}

#[derive(Serialize)]
pub struct LoginResponse{
    pub identity:auth_user::Identity,
    pub account :Option<AccountResonse>,
    pub consumer:Option<Consumer>
}

#[derive(Serialize)]
pub struct AccountResonse{
    #[serde(flatten)]
    pub account :Account,
    pub merchant:Merchant
}

pub async fn login_by_username(State(pool):State<AxumPgPool>,mut auth: AuthSession<AxumPgPool, AxumPgPool,User>,Json(req):Json<LoginRequest>)->Result<Json<LoginResponse>,(StatusCode,String)>{
    let mut conn=pool.pool.get().unwrap();//TODO error

    let login_info=get_login_info(req.username,&mut *conn).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    
    verify_password(login_info.user_id, req.password,&mut *conn).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;

    let user=users::dsl::users
            .filter(users::dsl::user_id.eq(login_info.user_id))
            .filter(users::dsl::enabled.eq(true))
            .get_result::<User>(&mut *conn).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;

    let account=accounts::table
        .inner_join(merchants::table.on(accounts::merchant_id.eq(merchants::merchant_id)))
        .filter(accounts::dsl::user_id.eq(user.user_id))
        .filter(accounts::dsl::enabled.eq(true))
        .get_result::<(Account,Merchant)>(&mut *conn)
        .ok() //TODO track error
        .map(|a_m| AccountResonse{
            account:a_m.0,
            merchant:a_m.1,
        });
    let consumer=consumers::dsl::consumers
        .filter(consumers::dsl::user_id.eq(user.user_id))
        .filter(consumers::dsl::enabled.eq(true))
        .get_result::<Consumer>(&mut *conn)
        .ok();
    
    let response=LoginResponse{
        identity:User::load_identity(login_info.user_id,pool.clone()),
        account,
        consumer,
    };
    auth.sign_in(login_info.user_id).await;
    
    Ok(Json(response))
}

pub async fn logout(mut auth: AuthSession< AxumPgPool, AxumPgPool,User>){
    auth.sign_out().await;
}

pub async fn get_current_identity(auth: AuthSession<AxumPgPool, AxumPgPool,User>)->Result<Json<auth_user::Identity>,(StatusCode,String)>{
    let identity=auth.identity.ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;
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
    State(pool):State<AxumPgPool>,
    Query(params):Query<PaginatedListRequest>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<Json<PaginatedListResponse<Consumer>>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::ACCOUNT])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error

    let get_consumers_query=|p:&PaginatedListRequest|{
        let mut query=consumers::dsl::consumers
            .filter(consumers::dsl::enabled.eq(true))
            .into_boxed();
        if let Some(key)=p.key.as_ref(){
            query=query
                .filter(consumers::dsl::cellphone.ilike(format!("{key}%")).or(consumers::dsl::real_name.ilike(format!("{key}%"))));   
        }
        query
    };

    let count=get_consumers_query(&params).count().get_result(&mut *conn).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    let data=get_consumers_query(&params)
        .order(consumers::dsl::create_time.desc())
        .limit(params.page_size)
        .offset(params.page_index*params.page_size)
        .get_results::<Consumer>(&mut *conn)
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    
    Ok(Json(PaginatedListResponse{
        page_index:params.page_index,
        page_size:params.page_size,
        total_count:count,
        data:data,
    }))
}

pub async fn add_consumer(
    State(pool):State<AxumPgPool>,
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
    Json(req): Json<ConsumerRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::ACCOUNT])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
        
    let mut conn=pool.pool.get().unwrap();//TODO error
    
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
    .execute(&mut *conn).map_err(|e|{
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
    .execute(&mut *conn).map_err(|e|{
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
    .execute(&mut *conn).map_err(|e|{
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
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

    Ok(())
}

pub async fn delete_consumer(
    State(pool):State<AxumPgPool>,
    Path(id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<(),(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::ACCOUNT])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error

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
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

    if count!=1 {
        return Err((StatusCode::NOT_FOUND,"data not exists".to_string()));
    }

    Ok(())
}

pub async fn update_consumer(
    State(pool):State<AxumPgPool>,
    Path(id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
    Json(req): Json<ConsumerRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::ACCOUNT])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
   
    let mut conn=pool.pool.get().unwrap();//TODO error

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
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;
    
    Ok(())
}

pub async fn get_consumer(
    State(pool):State<AxumPgPool>,
    Path(id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<Json<Consumer>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::ACCOUNT])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;

    let mut conn=pool.pool.get().unwrap();//TODO error  
    
    let consumer=consumers::dsl::consumers
        .filter(consumers::dsl::consumer_id.eq(id))
        .filter(consumers::dsl::enabled.eq(true))
        .get_result::<Consumer>(&mut *conn)
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    
    Ok(Json(consumer))
}
