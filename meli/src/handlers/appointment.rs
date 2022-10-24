use axum::{http::StatusCode, Json, extract::{Query, Path, State}};
use axum_session_authentication_middleware::session::AuthSession;
use bigdecimal::BigDecimal;
use chrono::{Local, DateTime};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
use random_color::{Color, Luminosity, RandomColor};
use crate::{
    schema::*,
    models::{Order, NewOrder, Member, Barber, ServiceType}, authorization_policy, constant
};
use diesel::prelude::*;
use crate::{models::User, axum_pg_pool::AxumPgPool};

use super::Search;

#[derive(Deserialize)]
pub struct AppointmentRequest{
    pub start_time:DateTime<Local>,
    pub end_time:DateTime<Local>,
    pub service_type_id:Uuid,
    pub barber_id:Uuid,
    pub member_id:Option<Uuid>,

    pub payment_type:String, // member/cash
    pub amount:BigDecimal,
    pub remark:Option<String>,
}

#[derive(Deserialize)]
pub struct CalendarRequest{
    pub start_date:DateTime<Local>,
    pub end_date:DateTime<Local>,
}

#[derive(Serialize)]
pub struct Event{
    #[serde(rename = "allDay")]
    pub all_day:bool,//false
   
    pub title:String,
    pub editable:bool,//false
    #[serde(rename = "startEditable")]
    pub start_editable:bool,//false
    pub display:String,//'auto' or 'background'

    #[serde(rename = "backgroundColor")]
    pub background_color:String,

    #[serde(rename = "extendedProps")]
    pub extended_props:Value,//{}
    #[serde(flatten)]
    pub order:Order,
}

pub async fn get_appointments(
    State(pool):State<AxumPgPool>,
    Query(params):Query<CalendarRequest>, 
    Query(search):Query<Search>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<Json<Vec<Event>>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error
  
    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID))
        .unwrap();

    let mut query=orders::table
        .left_join(members::table.on(members::member_id.nullable().eq(orders::member_id)))
        .inner_join(barbers::table.on(orders::barber_id.eq(barbers::barber_id)))
        .inner_join(service_types::table.on(orders::service_type_id.eq(service_types::service_type_id)))
        .filter(members::enabled.eq(true))
        .filter(barbers::enabled.eq(true))
        .filter(service_types::enabled.eq(true))
        .filter(orders::enabled.eq(true))
        .filter(orders::merchant_id.eq(merchant_id))
        .filter(orders::end_time.ge(params.start_date).and(orders::start_time.lt(params.end_date)))
        .into_boxed();

    if let Some(barber_id)=search.barber_id{
        query=query.filter(orders::barber_id.eq(barber_id))
    }

    let data= query.order(orders::create_time.desc())
        .get_results::<(Order,Option<Member>,Barber,ServiceType)>(&mut *conn)
        .map(|v|v.into_iter().map(|t|Event{
            all_day:false,
            editable:false,
            start_editable:false,
            background_color:RandomColor::new().to_rgb_string(), // rgb(139, 218, 232)
            display:"auto".into(),
            title: "".into(),
            extended_props:json!({
                "id":t.0.id,
                "customer": if let Some(m)=t.1 {m.real_name.unwrap_or("-".into())} else {t.0.consumer_type.clone()},
                "serviceName": t.3.name,
                "barberName":t.2.real_name.unwrap_or("-".into()),
            }),
            order:t.0,
        }).collect())
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    
    Ok(Json(data))
}

pub async fn add_appointment(
    State(pool):State<AxumPgPool>,
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
    Json(req): Json<AppointmentRequest>
)->Result<Json<Event>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
    .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error

    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID))
        .unwrap();

    let new_appointment=NewOrder{
        order_id: &Uuid::new_v4(),
        start_time:req.start_time,
        end_time:req.end_time,
        merchant_id:&merchant_id,
        consumer_type:if req.member_id.is_none() { "walk-in" } else { "member" },
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

    let event=orders::table
        .left_join(members::table.on(members::member_id.nullable().eq(orders::member_id)))
        .inner_join(barbers::table.on(orders::barber_id.eq(barbers::barber_id)))
        .inner_join(service_types::table.on(orders::service_type_id.eq(service_types::service_type_id)))
        .filter(orders::enabled.eq(true))
        .filter(orders::merchant_id.eq(merchant_id))
        .filter(orders::order_id.eq(new_appointment.order_id))
        .get_result::<(Order,Option<Member>,Barber,ServiceType)>(&mut *conn)
        .map(|t|Event{
            all_day:false,
            editable:false,
            start_editable:false,
            background_color:RandomColor::new().to_rgb_string(), // rgb(139, 218, 232)
            display:"auto".into(),
            title: "".into(),
            extended_props:json!({
                "id":t.0.id,
                "memberId": t.1.as_ref().map(|m|m.member_id),
                "customer": if let Some(m)=t.1 {m.real_name.unwrap_or("-".into())} else {t.0.consumer_type.clone()},
                "serviceName": t.3.name,
                "barberName":t.2.real_name.unwrap_or("-".into()),
                "startTime":t.0.start_time,
                "endTime":t.0.end_time,
                "remark":t.0.remark,
                "amount":t.0.amount,
                "total_minutes":(t.0.end_time-t.0.start_time).num_minutes(),
            }),
            order:t.0,
        })
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    Ok(Json(event))
}

pub async fn get_appointment(
    State(pool):State<AxumPgPool>,
    Path(appointment_id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<Json<Event>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;

    let mut conn=pool.pool.get().unwrap();//TODO error  
    
    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID))
        .unwrap();

    let event=orders::table
        .left_join(members::table.on(members::member_id.nullable().eq(orders::member_id)))
        .inner_join(barbers::table.on(orders::barber_id.eq(barbers::barber_id)))
        .inner_join(service_types::table.on(orders::service_type_id.eq(service_types::service_type_id)))
        .filter(orders::enabled.eq(true))
        .filter(orders::merchant_id.eq(merchant_id))
        .filter(orders::order_id.eq(appointment_id))
        .get_result::<(Order,Option<Member>,Barber,ServiceType)>(&mut *conn)
        .map(|t|Event{
            all_day:false,
            editable:false,
            start_editable:false,
            background_color:RandomColor::new().to_rgb_string(), // rgb(139, 218, 232)
            display:"auto".into(),
            title: "".into(),
            extended_props:json!({
                "id":t.0.id,
                "memberId": t.1.as_ref().map(|m|m.member_id),
                "customer": if let Some(m)=t.1 {m.real_name.unwrap_or("-".into())} else {t.0.consumer_type.clone()},
                "serviceName": t.3.name,
                "barberName":t.2.real_name.unwrap_or("-".into()),
                "startTime":t.0.start_time,
                "endTime":t.0.end_time,
                "remark":t.0.remark,
                "amount":t.0.amount,
                "total_minutes":(t.0.end_time-t.0.start_time).num_minutes(),
            }),
            order:t.0,
        })
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
        
    Ok(Json(event))
}
