use axum::{http::StatusCode, Json, extract::{Query, Path, State}};
use axum_session_authentication_middleware::session::AuthSession;
use chrono::Local;
use serde::Deserialize;
use bigdecimal::BigDecimal;
use uuid::Uuid;
use crate::{
    schema::*,
    models::{ServiceType, NewServiceType, Barber}, authorization_policy
};
use diesel::{
    prelude::*, // for .filter
    select, 
    dsl::exists,
}; 
use crate::{models::User, axum_pg_pool::AxumPgPool};
use super::{PaginatedListRequest,PaginatedListResponse};

#[derive(Deserialize)]
pub struct ServiceTypeRequest{
    pub name:String,
    pub normal_prize:BigDecimal,
    pub member_prize:BigDecimal,
    pub estimated_duration: i32,
}

pub async fn get_service_types(
    State(pool):State<AxumPgPool>,
    Query(params):Query<PaginatedListRequest>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<Json<PaginatedListResponse<ServiceType>>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error

    let barber=serde_json::from_str::<Option<Barber>>(auth.axum_session.lock().unwrap().get_data("barber"))
	    .unwrap().unwrap();

    let get_service_types_query=|p:&PaginatedListRequest|{
        let mut query=service_types::dsl::service_types
            .filter(service_types::dsl::enabled.eq(true))
            .filter(service_types::dsl::merchant_id.eq(barber.merchant_id))
            .into_boxed();
        
        if let Some(key)=p.key.as_ref() {
            if key.len()>0 {
                query=query
                .filter(service_types::dsl::name.ilike(format!("%{key}%")));   
            }
        }
        query
    };

    let count=get_service_types_query(&params).count().get_result(&mut *conn).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    let data=get_service_types_query(&params)
        .order(service_types::dsl::create_time.desc())
        .limit(params.page_size)
        .offset(params.page_index*params.page_size)
        .get_results::<ServiceType>(&mut *conn)
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    
    Ok(Json(PaginatedListResponse{
        page_index:params.page_index,
        page_size:params.page_size,
        total_count:count,
        data:data,
    }))
}

pub async fn add_service_type(
    State(pool):State<AxumPgPool>,
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
    Json(req): Json<ServiceTypeRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error
    
    let barber=serde_json::from_str::<Option<Barber>>(auth.axum_session.lock().unwrap().get_data("barber"))
	    .unwrap().unwrap();

    let existed=select(exists(service_types::dsl::service_types
        .filter(service_types::dsl::enabled.eq(true))
        .filter(service_types::dsl::name.eq(&req.name))
        .filter(service_types::dsl::merchant_id.eq(barber.merchant_id))))
        .get_result::<bool>(&mut *conn)
        .ok();

    if let Some(true)=existed{
        return Err((StatusCode::INTERNAL_SERVER_ERROR,"已存在该服务名称".to_string()));
    } else {
        let new_service_type=NewServiceType{
            service_type_id: &Uuid::new_v4(),
            merchant_id:&barber.merchant_id,
            name:&req.name,
            normal_prize:&req.normal_prize,
            member_prize:&req.member_prize,
            estimated_duration:req.estimated_duration,
            enabled:true,
            create_time: Local::now(),
            update_time: Local::now(),
            data: None,
        };
        diesel::insert_into(service_types::table)
        .values(&new_service_type)
        .execute(&mut *conn).map_err(|e|{
            tracing::error!("{}",e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
        })?;
    }
    
    Ok(())
}

pub async fn delete_service_type(
    State(pool):State<AxumPgPool>,
    Path(service_type_id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<(),(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error

    let barber=serde_json::from_str::<Option<Barber>>(auth.axum_session.lock().unwrap().get_data("barber"))
	    .unwrap().unwrap();

    let count=diesel::update(
        service_types::dsl::service_types
        .filter(service_types::dsl::service_type_id.eq(service_type_id))
        .filter(service_types::dsl::merchant_id.eq(barber.merchant_id))
        .filter(service_types::dsl::enabled.eq(true))
    )
    .set((
        service_types::dsl::enabled.eq(false),
        service_types::dsl::update_time.eq(Local::now())
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

pub async fn update_service_type(
    State(pool):State<AxumPgPool>,
    Path(service_type_id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
    Json(req): Json<ServiceTypeRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
   
    let mut conn=pool.pool.get().unwrap();//TODO error

    let barber=serde_json::from_str::<Option<Barber>>(auth.axum_session.lock().unwrap().get_data("barber"))
	    .unwrap().unwrap();

    let num=diesel::update(
        service_types::dsl::service_types
        .filter(service_types::dsl::service_type_id.eq(service_type_id))
        .filter(service_types::dsl::merchant_id.eq(barber.merchant_id))
        .filter(service_types::dsl::enabled.eq(true))
    )
    .set((
            service_types::dsl::name.eq(req.name),
            service_types::dsl::normal_prize.eq(req.normal_prize),
            service_types::dsl::member_prize.eq(req.member_prize),
            service_types::dsl::estimated_duration.eq(req.estimated_duration),
            service_types::dsl::update_time.eq(Local::now())
        ))
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

    if num !=1 {
        tracing::error!("update_service_type affected num: {}",num);
    }
    
    Ok(())
}

pub async fn get_service_type(
    State(pool):State<AxumPgPool>,
    Path(service_type_id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<Json<ServiceType>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;

    let mut conn=pool.pool.get().unwrap();//TODO error  
    
    let barber=serde_json::from_str::<Option<Barber>>(auth.axum_session.lock().unwrap().get_data("barber"))
	    .unwrap().unwrap();

    let service_type=service_types::dsl::service_types
        .filter(service_types::dsl::enabled.eq(true))
        .filter(service_types::dsl::service_type_id.eq(service_type_id))
        .filter(service_types::dsl::merchant_id.eq(barber.merchant_id))
        .get_result::<ServiceType>(&mut *conn)
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
        
    Ok(Json(service_type))
}
