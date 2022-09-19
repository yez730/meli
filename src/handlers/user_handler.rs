use axum::{http::Method, Json, extract::{Query, Path}};
use axum_sessions_auth::{Auth, Rights, AuthSession};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    schema::*,
    models::{Account, Merchant, Consumer}, authorization_policy, my_date_format
};
use diesel::{
    prelude::*, // for .filter
    data_types::Cents
}; 

use crate::{models::User, axum_pg_pool::AxumPgPool, utils::{get_connection}, login_managers::{get_login_info, password_login::verify_password}};

use super::{Response, PaginatedListRequest,PaginatedListResponse};

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
    pub permissions:String,
    pub roles:String,
    pub merchant_id:Uuid,
    pub merchant_name:String,
}

pub async fn login_by_username(auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>,Json(req):Json<LoginRequest>,)->Json<Response<LoginResponse>>{
    let login_info=get_login_info(req.username);
    let login_info=match login_info {
        Some(login_info)=>login_info,
        None=> return Json(Response::fail(format!("login info not exists."))),
    };

    if !verify_password(login_info.user_id, req.password){
        return Json(Response::fail(format!("username password doesn't match.")))
    }
    
    let user=users::dsl::users
            .filter(users::dsl::user_id.eq(login_info.user_id))
            .filter(users::dsl::enabled.eq(true))
            .get_result::<User>(&mut get_connection());
    let user=match user {
        Ok(user)=>user,
        Err(e)=>{
            tracing::debug!("get user {} error {}.",login_info.user_id,e.to_string());
            return Json(Response::fail(format!("get user error.")));
        }
    };

    let account=accounts::dsl::accounts
        .filter(accounts::dsl::user_id.eq(login_info.user_id))
        .filter(accounts::dsl::enabled.eq(true))
        .get_result::<Account>(&mut get_connection());
    let account=match account {
        Ok(account)=>account,
        Err(e)=>{
            tracing::debug!("get account {} error {}.",login_info.user_id,e.to_string());
            return Json(Response::fail(format!("get account error.")));
        }
    };

    let merchant=merchants::dsl::merchants
        .filter(merchants::dsl::merchant_id.eq(account.merchant_id))
        .filter(merchants::dsl::enabled.eq(true))
        .get_result::<Merchant>(&mut get_connection());
    let merchant=match merchant {
        Ok(merchant)=>merchant,
        Err(e)=>{
            tracing::debug!("get merchant {} error {}.",account.merchant_id,e.to_string());
            return Json(Response::fail(format!("get merchant error.")));
        }
    };
    
    // set user to cookie
    auth.login_user(user.user_id).await;
    auth.remember_user(true).await;

    let login_response=LoginResponse{
        account_id:account.account_id,
        cellphone:account.cellphone,
        real_name:account.real_name.unwrap_or("no setting".to_string()),
        permissions:user.permissions,
        roles:user.roles,
        merchant_id:merchant.merchant_id,
        merchant_name:merchant.merchant_name,
    };
    Json(Response::succeed(login_response))
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
    
    #[serde(default, with = "my_date_format")]
    pub birth_day:Option<DateTime<Local>>,
}

#[derive(Serialize)]
pub struct ConsumerResponse{
    pub user_id: Uuid,
    pub consumer_id: Uuid,
    pub cellphone:String,
    pub real_name:String,//StringOption<String>,
    pub gender:String,//Option<String>,
    pub birth_day:String,//Option<DateTime<Local>>,
    pub balance:String,//Option<Cents>, // Cents has no [Serialize]
    pub create_time: String,//chrono::DateTime<Local>,
    pub update_time: String,//chrono::DateTime<Local>,
}

pub async fn get_consumers(Query(params):Query<PaginatedListRequest>, method: Method, auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>)->Json<Response<PaginatedListResponse<ConsumerResponse>>>{
    match auth.current_user {
        Some(cur_user) =>  {
            if !Auth::<User, Uuid, AxumPgPool>::build([Method::GET], false)
                .requires(Rights::any([
                    Rights::permission(authorization_policy::ACCOUNT)
                ]))
                .validate(&cur_user, &method, None)
                .await
            {
                return Json(Response::fail("no permission.".to_string()));
            }
            let _s=params.key.map_or(true, |k|k=="");
            let merchant=consumers::dsl::consumers
            // .or_filter(params.key.map_or(true, |k|cellphone.eq(k)))
            // .or_filter(params.key.map_or(true, |k|email.eq(k)))
            // .or_filter(params.key.map_or(true, |k|real_name.eq(k)))
            .get_results::<Consumer>(&mut get_connection());
            //TODO 分页
    
            Json(Response::fail("msg.".to_string()))
        }
        None=>Json(Response::fail("no login, login first.".to_string()))
    }
}

pub async fn add_consumer(method: Method, auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>,Json(req): Json<ConsumerRequest>)->Json<Response<()>>{
    match auth.current_user {
        Some(cur_user) =>  {
            if !Auth::<User, Uuid, AxumPgPool>::build([Method::POST], false)
                .requires(Rights::any([
                    Rights::permission(authorization_policy::ACCOUNT)
                ]))
                .validate(&cur_user, &method, None)
                .await
            {
                return Json(Response::fail("no permission.".to_string()));
            }
            
            //添加 
            // 1. add user [DEFAULT_PERMISSIONS_OF_CONSUMER] // now '[]'
            // 2. add login info /login info provider(//TODO cellphone login info provider)
            // 3. add comsumer.
    
            Json(Response::succeed_with_empty())
        }
        None=>Json(Response::fail("no login, login first.".to_string()))
    }
}

pub async fn delete_consumer(Path(id):Path<Uuid>, method: Method, auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>)->Json<Response<()>>{
    match auth.current_user {
        Some(cur_user) =>  {
            if !Auth::<User, Uuid, AxumPgPool>::build([Method::DELETE], false)
                .requires(Rights::any([
                    Rights::permission(authorization_policy::ACCOUNT)
                ]))
                .validate(&cur_user, &method, None)
                .await
            {
                return Json(Response::fail("no permission.".to_string()));
            }

            // 先不实际删除数据
            let result=diesel::update(
                consumers::dsl::consumers
                .filter(consumers::dsl::consumer_id.eq(id))
                .filter(consumers::dsl::enabled.eq(true))
            )
            .set((
                    consumers::dsl::enabled.eq(false),
                    consumers::dsl::update_time.eq(Local::now())
                ))
            .execute(&mut get_connection());

            match result {
                Ok(_)=>Json(Response::succeed_with_empty()),
                Err(e)=>Json(Response::fail(format!("delete consumber {} failed {}",id,e.to_string()))),
            }
        }
        None=>Json(Response::fail("no login, login first.".to_string()))
    }
}

pub async fn update_consumer(Path(id):Path<Uuid>, method: Method, auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>,Json(req): Json<ConsumerRequest>)->Json<Response<()>>{
    match auth.current_user {
        Some(cur_user) =>  {
            if !Auth::<User, Uuid, AxumPgPool>::build([Method::POST], false)
                .requires(Rights::any([
                    Rights::permission(authorization_policy::ACCOUNT)
                ]))
                .validate(&cur_user, &method, None)
                .await
            {
                return Json(Response::fail("no permission.".to_string()));
            }
            
            let result=diesel::update(
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
                .execute(&mut get_connection());
            
            match result {
                Ok(_)=>Json(Response::succeed_with_empty()),
                Err(e)=>{
                    tracing::debug!("update consumber {} failed {}",id,e.to_string());
                    Json(Response::fail(format!("update consumber {} failed {}",id,e.to_string())))
                },
            }
        }
        None=>Json(Response::fail("no login, login first.".to_string()))
    }
}
