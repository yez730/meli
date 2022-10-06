use axum::{http::StatusCode, Json, extract::{Query, Path, State}};
use axum_session_authentication_middleware::session::AuthSession;
use bigdecimal::BigDecimal;
use chrono::{Local, NaiveDate, NaiveTime};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    schema::*,
    models::{Order, NewUser, NewOrder, NewLoginInfo, NewPasswordLoginProvider}, authorization_policy
};
use diesel::{
    prelude::*, // for .filter
    select, 
    dsl::exists,
}; 
use crate::{models::User, axum_pg_pool::AxumPgPool};
use super::{PaginatedListRequest,PaginatedListResponse};

#[derive(Deserialize)]
pub struct AppointmentRequest{
    pub date:NaiveDate,
    pub start_time:NaiveTime,
    pub end_time:NaiveTime,
    pub service_type_id:Uuid,
    pub barber_id:Uuid,
    pub member_id:Option<Uuid>,

    pub payment_type:String, // member/cash
    pub amount:BigDecimal,
    pub remark:Option<String>,
}

#[derive(Deserialize)]
pub struct CalendarRequest{
    pub start_date:NaiveDate,
    pub end_date:NaiveDate,
}

pub async fn get_appointments(
    State(pool):State<AxumPgPool>,
    Path(merchant_id):Path<Uuid>, 
    Query(params):Query<CalendarRequest>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<Json<Vec<Order>>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error
  
    let data=orders::dsl::orders
        .filter(orders::dsl::enabled.eq(true))
        .filter(orders::dsl::merchant_id.eq(merchant_id))
        .filter(orders::dsl::date.ge(params.start_date).and(orders::dsl::date.lt(params.end_date)))
        .order(orders::dsl::create_time.desc())
        .get_results::<Order>(&mut *conn)
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    
    Ok(Json(data))
}

pub async fn add_appointment(
    State(pool):State<AxumPgPool>,
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
    Path(merchant_id):Path<Uuid>, 
    Json(req): Json<AppointmentRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
    .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error

    let exist_merchant=select(exists(
        merchants::dsl::merchants
        .filter(merchants::dsl::enabled.eq(true))
        .filter(merchants::dsl::merchant_id.eq(&merchant_id))
    ))
    .get_result::<bool>(&mut *conn)
    .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"get_result error".to_string()))?;

    if !exist_merchant{
        return Err((StatusCode::INTERNAL_SERVER_ERROR,"商户不存在".to_string()));
    }

    let new_appointment=NewOrder{
        order_id: &Uuid::new_v4(),
        date: &req.date,
        start_time:&req.start_time,
        end_time:&req.end_time,
        merchant_id:&merchant_id,
        consumer_type:if req.member_id.is_none(){"walk-in" } else {"member"},
        member_id:req.member_id.as_ref(),
        barber_id:&req.barber_id,
        service_type_id:&req.service_type_id,
        status:"Completed",
        payment_type:&req.payment_type,
        amount:&req.amount,
        remark:req.remark.as_deref(),
       
        enabled:true,
        create_time: Local::now(),
        update_time: Local::now(),
        data: None,
    };
    diesel::insert_into(orders::table)
    .values(&new_appointment)
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;
    
    Ok(())
}

pub async fn get_appointment(
    State(pool):State<AxumPgPool>,
    Path((merchant_id,appointment_id)):Path<(Uuid,Uuid)>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<Json<Order>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;

    let mut conn=pool.pool.get().unwrap();//TODO error  
    
    let appointment=orders::dsl::orders
        .filter(orders::dsl::enabled.eq(true))
        .filter(orders::dsl::merchant_id.eq(merchant_id))
        .filter(orders::dsl::order_id.eq(appointment_id))
        .get_result::<Order>(&mut *conn)
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
        
    Ok(Json(appointment))
}
