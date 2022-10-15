use axum::{http::StatusCode, Json, extract::{Query, Path, State}};
use axum_session_authentication_middleware::session::AuthSession;
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    schema::*,
    models::{Barber, NewUser, NewBarber, NewLoginInfo, NewPasswordLoginProvider}, authorization_policy
};
use diesel::{
    prelude::*, // for .filter
    select, 
    dsl::exists,
}; 
use crate::{models::User, axum_pg_pool::AxumPgPool};
use super::{PaginatedListRequest,PaginatedListResponse};

#[derive(Deserialize)]
pub struct BarberRequest{
    pub cellphone:String,
    pub real_name:Option<String>,
    pub email:Option<String>,
}

pub async fn get_barbers(
    State(pool):State<AxumPgPool>,
    Query(params):Query<PaginatedListRequest>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<Json<PaginatedListResponse<Barber>>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error

    let barber=serde_json::from_str::<Option<Barber>>(auth.axum_session.lock().unwrap().get_data("barber"))
	    .unwrap().unwrap();

    let get_barbers_query=|p:&PaginatedListRequest|{
        let mut query=barbers::dsl::barbers
            .filter(barbers::dsl::enabled.eq(true))
            .filter(barbers::dsl::merchant_id.eq(barber.merchant_id))
            .into_boxed();
        if let Some(key)=p.key.as_ref(){
            if key.len()>0 {
                query=query
                    .filter(barbers::dsl::cellphone.ilike(format!("%{key}%")).or(barbers::dsl::real_name.ilike(format!("%{key}%"))));   
            }
        }
        query
    };

    let count=get_barbers_query(&params).count().get_result(&mut *conn).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    let data=get_barbers_query(&params)
        .order(barbers::dsl::create_time.desc())
        .limit(params.page_size)
        .offset(params.page_index*params.page_size)
        .get_results::<Barber>(&mut *conn)
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    
    Ok(Json(PaginatedListResponse{
        page_index:params.page_index,
        page_size:params.page_size,
        total_count:count,
        data:data,
    }))
}

pub async fn add_barber(
    State(pool):State<AxumPgPool>,
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
    Json(req): Json<BarberRequest>
)->Result<Json<Barber>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error
    
    let barber=serde_json::from_str::<Option<Barber>>(auth.axum_session.lock().unwrap().get_data("barber"))
	    .unwrap().unwrap();

    //添加 TODO insert data with enabled settting false, finally set to true.
    let existed=select(exists(barbers::dsl::barbers
        .filter(barbers::dsl::enabled.eq(true))
        .filter(barbers::dsl::cellphone.eq(&req.cellphone))))
        .get_result::<bool>(&mut *conn)
        .ok();

    if let Some(true)=existed{
        return Err((StatusCode::INTERNAL_SERVER_ERROR,"已存在该用户".to_string()));
    } else {
        // 1. add user
        let user_id=Uuid::new_v4();
        let new_user=NewUser{
            user_id: &user_id,
            description: "test user",
            permissions:&serde_json::to_string(authorization_policy::DEFAULT_PERMISSIONS_OF_MERCHANT_BARBER).unwrap(),
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
            login_info_barber: &req.cellphone,
            login_info_type: "Username", //TODO get enum variant value string
            user_id: &user_id,
            enabled: true, // TODO false
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

        // 3. add barber
        let new_barber=NewBarber{
            user_id:  &user_id,
            barber_id: &Uuid::new_v4(),
            merchant_id:&barber.merchant_id,
            email:req.email.as_deref(),
            cellphone:&req.cellphone,
            real_name:req.real_name.as_deref(),
            enabled:true,
            create_time: Local::now(),
            update_time: Local::now(),
            data: None,
        };
        diesel::insert_into(barbers::table)
        .values(&new_barber)
        .execute(&mut *conn).map_err(|e|{
            tracing::error!("{}",e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
        })?;

        let barber=barbers::dsl::barbers
            .filter(barbers::dsl::enabled.eq(true))
            .filter(barbers::dsl::barber_id.eq(new_barber.barber_id))
            .get_result::<Barber>(&mut *conn)
            .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
        
        Ok(Json(barber))
    }
}

pub async fn delete_barber(
    State(pool):State<AxumPgPool>,
    Path(barber_id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<(),(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error

    let count=diesel::update(
        barbers::dsl::barbers
        .filter(barbers::dsl::barber_id.eq(barber_id))
        .filter(barbers::dsl::enabled.eq(true))
    )
    .set((
        barbers::dsl::enabled.eq(false),
        barbers::dsl::update_time.eq(Local::now())
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

pub async fn update_barber(
    State(pool):State<AxumPgPool>,
    Path(barber_id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
    Json(req): Json<BarberRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
   
    let mut conn=pool.pool.get().unwrap();//TODO error

    let num=diesel::update(
        barbers::dsl::barbers
        .filter(barbers::dsl::barber_id.eq(barber_id))
        .filter(barbers::dsl::enabled.eq(true))
    )
    .set((
            barbers::dsl::cellphone.eq(req.cellphone),
            barbers::dsl::real_name.eq(req.real_name),
            barbers::dsl::email.eq(req.email),
            barbers::dsl::update_time.eq(Local::now())
        ))
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

    if num !=1 {
        tracing::error!("update_barber affected num: {}",num);
    }
    
    Ok(())
}

pub async fn get_barber(
    State(pool):State<AxumPgPool>,
    Path(barber_id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<Json<Barber>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::MERCHANT_ADMINISTRATOR])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;

    let mut conn=pool.pool.get().unwrap();//TODO error  
    
    let barber=barbers::dsl::barbers
        .filter(barbers::dsl::enabled.eq(true))
        .filter(barbers::dsl::barber_id.eq(barber_id))
        .get_result::<Barber>(&mut *conn)
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
        
    Ok(Json(barber))
}
