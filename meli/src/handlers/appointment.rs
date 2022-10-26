use axum::{http::StatusCode, Json, extract::{Query, Path, State}};
use axum_session_authentication_middleware::session::AuthSession;
use bigdecimal::BigDecimal;
use chrono::{Local, DateTime};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
use random_color::RandomColor;
use crate::{
    schema::*,
    models::{Order, NewOrder, Member, Barber, ServiceType}, authorization_policy, constant
};
use diesel::prelude::*;
use crate::{models::User, axum_pg::AxumPg};

use super::Search;

#[derive(Deserialize)]
pub struct AppointmentRequest{
    #[serde(rename ="startTime")]
    pub start_time:DateTime<Local>,

    #[serde(rename ="endTime")]
    pub end_time:DateTime<Local>,

    #[serde(rename ="serviceTypeId")]
    pub service_type_id:Uuid,

    #[serde(rename ="barberId")]
    pub barber_id:Uuid,

    #[serde(rename ="memberId")]
    pub member_id:Option<Uuid>,

    #[serde(rename ="paymentType")]
    pub payment_type:String, // member/cash

    pub amount:BigDecimal,
    pub remark:Option<String>,
}

#[derive(Deserialize)]
pub struct CalendarRequest{
    #[serde(rename ="startDate")]
    pub start_date:DateTime<Local>,

    #[serde(rename ="endDate")]
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
    pub extended_props:Value,//需为json对象

    #[serde(flatten)]
    pub order:Order,
}

pub async fn get_appointments(
    State(pg):State<AxumPg>,
    Query(params):Query<CalendarRequest>, 
    Query(search):Query<Search>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<Json<Vec<Event>>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();
  
    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

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
    State(pg):State<AxumPg>,
    auth: AuthSession<AxumPg, AxumPg,User>,
    Json(req): Json<AppointmentRequest>
)->Result<Json<Event>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();

    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

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
    State(pg):State<AxumPg>,
    Path(appointment_id):Path<Uuid>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<Json<Event>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;

    let mut conn=pg.pool.get().unwrap();
    
    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

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
