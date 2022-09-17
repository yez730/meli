use axum::{http::Method, Json, extract::{Query, Path}};
use axum_sessions_auth::{Auth, Rights, AuthSession};
use chrono::Local;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    schema::{
        users::dsl::{users,user_id}, 
        accounts::dsl::{
            accounts,
            user_id as account_user_id
        }, 
        merchants::dsl::*, 
        consumers::dsl::{*,credential_no as c_credential_no,real_name as c_real_name}, 
    }, 
    models::{Account, Merchant, Consumer}, authorization_policy
};
use diesel::prelude::*; // for .filter

use crate::{models::User, axum_pg_pool::AxumPgPool, util::{ get_connection}, login_managers::{get_login_info, password_login::verify_password}};

#[derive(Serialize)]
pub struct Response<T:Serialize>{
    pub succeeded :bool,
    pub message:String,
    pub data:Option<T>,
}

impl<T:Serialize> Response<T>{
    pub fn fail(msg:String)->Response<T>{
        Response{
            succeeded:false,
            message:msg,
            data:None,
        }
    }

    pub fn succeed(d:T)->Response<T>{
        Response{
            succeeded:true,
            message:"operation success".to_string(),
            data:Some(d),
        }
    }

    pub fn succeed_with_empty()->Response<T>{
        Response{
            succeeded:true,
            message:"operation success".to_string(),
            data:None,
        }
    }
}

#[derive(Deserialize)]
pub struct PaginatedListRequest {
    //分页索引，从 0 开始
    page_index:i32,

    //分页大小
    page_size:i32,

    //搜索框
    key:Option<String>,
}

#[derive(Serialize)]
pub struct PaginatedListResponse<T:Serialize> {
    //分页索引，从 0 开始
    page_index:i32,

    //分页大小
    page_size:i32,

    //获取分页时原数据的元素总数量
    total_count:i32,

    //获取分页时原数据的元素总页数。
    total_page_count:i32,// = (int)Math.Ceiling(totalCount / (double)pageSize);

    data:Vec<T>,
}


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
    
    let user=users
            .filter(user_id.eq(login_info.user_id))
            .get_result::<User>(&mut get_connection());
    let user=match user {
        Ok(user)=>user,
        Err(e)=>{
            tracing::debug!("get user {} error {}.",login_info.user_id,e.to_string());
            return Json(Response::fail(format!("get user error.")));
        }
    };

    let account=accounts
        .filter(account_user_id.eq(login_info.user_id))
        .get_result::<Account>(&mut get_connection());
    let account=match account {
        Ok(account)=>account,
        Err(e)=>{
            tracing::debug!("get account {} error {}.",login_info.user_id,e.to_string());
            return Json(Response::fail(format!("get account error.")));
        }
    };

    let merchant=merchants
        .filter(merchant_id.eq(account.merchant_id))
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

#[derive(Serialize)]
pub struct ConsumerResponse{
    pub cellphone:String,
    pub email:String,
    pub real_name:String,
    pub create_time:String,
    //TODO 金额 、性别
}

#[derive(Deserialize)]
pub struct ConsumerRequest{
    pub cellphone:String,
    pub email:Option<String>,
    pub credential_no:Option<String>,
    pub real_name:Option<String>,
    
    //TODO 金额 、性别
}

pub async fn get_consumers(Query(params):Query<PaginatedListRequest>, method: Method, auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>)->Json<Response<PaginatedListResponse<ConsumerResponse>>>{
    match auth.current_user {
        Some(cur_user) =>  {
            if !Auth::<User, Uuid, AxumPgPool>::build([Method::GET], false)
                .requires(Rights::any([
                    Rights::permission(authorization_policy::KEHU_GUANLI)
                ]))
                .validate(&cur_user, &method, None)
                .await
            {
                return Json(Response::fail("no permission.".to_string()));
            }
            let _s=params.key.map_or(true, |k|k=="");
            let merchant=consumers
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
                    Rights::permission(authorization_policy::KEHU_GUANLI)
                ]))
                .validate(&cur_user, &method, None)
                .await
            {
                return Json(Response::fail("no permission.".to_string()));
            }
            
            //添加 
            // 1. add user [DEFAULT_PERMISSIONS_OF_CONSUMER]
            // 2. add login info /login info provider(//TODO cellphone login info provider)
            // 3. add comsumer.
            
    
            Json(Response::succeed_with_empty())
        }
        None=>Json(Response::fail("no login, login first.".to_string()))
    }
}

pub async fn delete_consumer(Path(c_id):Path<Uuid>, method: Method, auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>)->Json<Response<()>>{
    match auth.current_user {
        Some(cur_user) =>  {
            if !Auth::<User, Uuid, AxumPgPool>::build([Method::DELETE], false)
                .requires(Rights::any([
                    Rights::permission(authorization_policy::KEHU_GUANLI)
                ]))
                .validate(&cur_user, &method, None)
                .await
            {
                return Json(Response::fail("no permission.".to_string()));
            }
            
            let result=diesel::delete(consumers.filter(consumer_id.eq(c_id)))
            .execute(&mut get_connection());
            
            match result {
                Ok(_)=>Json(Response::succeed_with_empty()),
                Err(e)=>Json(Response::fail(format!("delete consumber {} failed {}",c_id,e.to_string()))),
            }
        }
        None=>Json(Response::fail("no login, login first.".to_string()))
    }
}

pub async fn update_consumer(Path(c_id):Path<Uuid>, method: Method, auth: AuthSession<User, Uuid, AxumPgPool, AxumPgPool>,Json(req): Json<ConsumerRequest>)->Json<Response<()>>{
    match auth.current_user {
        Some(cur_user) =>  {
            if !Auth::<User, Uuid, AxumPgPool>::build([Method::POST], false)
                .requires(Rights::any([
                    Rights::permission(authorization_policy::KEHU_GUANLI)
                ]))
                .validate(&cur_user, &method, None)
                .await
            {
                return Json(Response::fail("no permission.".to_string()));
            }
            
            let result=diesel::update(consumers.filter(consumer_id.eq(c_id)))
                .set((
                        cellphone.eq(req.cellphone),
                        email.eq(req.email),
                        c_credential_no.eq(req.credential_no),
                        c_real_name.eq(req.real_name)
                    ))
                .execute(&mut get_connection());
            
            match result {
                Ok(_)=>Json(Response::succeed_with_empty()),
                Err(e)=>Json(Response::fail(format!("update consumber {} failed {}",c_id,e.to_string()))),
            }
        }
        None=>Json(Response::fail("no login, login first.".to_string()))
    }
}
