use axum::{http::StatusCode, Json, extract::{Query, Path, State}};
use axum_session_authentication_middleware::session::AuthSession;
use chrono::Local;
use serde::Deserialize;
use bigdecimal::BigDecimal;
use uuid::Uuid;
use crate::{
    schema::*,
    models::{ServiceType, NewServiceType}, authorization_policy, constant
};
use diesel::{
    prelude::*, // for .filter
    select, 
    dsl::exists,
}; 
use crate::{models::User, axum_pg::AxumPg};
use super::{PaginatedListRequest,PaginatedListResponse, Search};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceTypeRequest{
    pub name:String,

    pub normal_prize:BigDecimal,

    pub member_prize:BigDecimal,

    pub estimated_duration: i32,
}

pub async fn get_service_types(
    State(pg):State<AxumPg>,
    Query(params):Query<PaginatedListRequest>, 
    Query(search):Query<Search>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<Json<PaginatedListResponse<ServiceType>>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();

    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let fn_get_service_types_query=||{
        let mut query=service_types::table
            .filter(service_types::enabled.eq(true))
            .filter(service_types::merchant_id.eq(merchant_id))
            .into_boxed();
        
        if let Some(key)=search.key.as_ref() {
            if key.len()>0 {
                query=query
                .filter(service_types::name.ilike(format!("%{key}%")));   
                }
            }
        query
    };

    let count=fn_get_service_types_query()
        .count()
        .get_result(&mut *conn)
        .unwrap();
    let data=fn_get_service_types_query()
        .order(service_types::create_time.desc())
        .limit(params.page_size)
        .offset(params.page_index*params.page_size)
        .get_results::<ServiceType>(&mut *conn)
        .unwrap();
    
    Ok(Json(PaginatedListResponse{
        page_index:params.page_index,
        page_size:params.page_size,
        total_count:count,
        data:data,
    }))
}

pub async fn add_service_type(
    State(pg):State<AxumPg>,
    auth: AuthSession<AxumPg, AxumPg,User>,
    Json(req): Json<ServiceTypeRequest>
)->Result<Json<ServiceType>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();
    
    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let existed=select(exists(service_types::table
        .filter(service_types::enabled.eq(true))
        .filter(service_types::name.eq(&req.name))
        .filter(service_types::merchant_id.eq(merchant_id))))
        .get_result::<bool>(&mut *conn)
        .ok();
    
    if let Some(true)=existed{
        return Err((StatusCode::INTERNAL_SERVER_ERROR,"已存在该服务名称".to_string()));
    } else {
        let new_service_type=NewServiceType{
            service_type_id: &Uuid::new_v4(),
            merchant_id:&merchant_id,
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
            .execute(&mut *conn)
            .unwrap();

        let service_type=service_types::table
            .filter(service_types::enabled.eq(true))
            .filter(service_types::service_type_id.eq(new_service_type.service_type_id))
            .filter(service_types::merchant_id.eq(merchant_id))
            .get_result::<ServiceType>(&mut *conn)
            .unwrap();
        
        Ok(Json(service_type))
    }
}

pub async fn delete_service_type(
    State(pg):State<AxumPg>,
    Path(service_type_id):Path<Uuid>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<(),(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();

    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let _existed=service_types::table
        .filter(service_types::service_type_id.eq(service_type_id))
        .filter(service_types::merchant_id.eq(merchant_id))
        .filter(service_types::enabled.eq(true))
        .get_result::<ServiceType>(&mut *conn)
        .map_err(|_|{
            (StatusCode::NOT_FOUND,"服务类型不存在".to_string())
        })?;

    diesel::update(
        service_types::table
        .filter(service_types::service_type_id.eq(service_type_id))
        .filter(service_types::merchant_id.eq(merchant_id))
        .filter(service_types::enabled.eq(true))
    )
    .set((
        service_types::enabled.eq(false),
        service_types::update_time.eq(Local::now())
    ))
    .execute(&mut *conn)
    .unwrap();

    Ok(())
}

pub async fn update_service_type(
    State(pg):State<AxumPg>,
    Path(service_type_id):Path<Uuid>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
    Json(req): Json<ServiceTypeRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
   
    let mut conn=pg.pool.get().unwrap();

    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let _existed=service_types::table
        .filter(service_types::service_type_id.eq(service_type_id))
        .filter(service_types::merchant_id.eq(merchant_id))
        .filter(service_types::enabled.eq(true))
        .get_result::<ServiceType>(&mut *conn)
        .unwrap();

    diesel::update(
        service_types::table
        .filter(service_types::service_type_id.eq(service_type_id))
        .filter(service_types::merchant_id.eq(merchant_id))
        .filter(service_types::enabled.eq(true))
    )
    .set((
        service_types::name.eq(req.name),
        service_types::normal_prize.eq(req.normal_prize),
        service_types::member_prize.eq(req.member_prize),
        service_types::estimated_duration.eq(req.estimated_duration),
        service_types::update_time.eq(Local::now())
    ))
    .execute(&mut *conn)
    .unwrap();
    
    Ok(())
}

pub async fn get_service_type(
    State(pg):State<AxumPg>,
    Path(service_type_id):Path<Uuid>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<Json<ServiceType>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;

    let mut conn=pg.pool.get().unwrap();
    
    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let service_type=service_types::table
        .filter(service_types::enabled.eq(true))
        .filter(service_types::service_type_id.eq(service_type_id))
        .filter(service_types::merchant_id.eq(merchant_id))
        .get_result::<ServiceType>(&mut *conn)
        .map_err(|_|(StatusCode::NOT_FOUND,"服务类型不存在".to_string()))?;
        
    Ok(Json(service_type))
}
