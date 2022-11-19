use axum::{http::StatusCode, Json, extract::{Query, Path, State}};
use axum_session_authentication_middleware::session::AuthSession;
use bigdecimal::BigDecimal;
use chrono::{Local, DateTime};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
use crate::{
    schema::*,
    models::*, 
    authorization_policy, 
    constant
};
use diesel::prelude::*;
use crate::{models::User, axum_pg::AxumPg};

use super::Search;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct CalendarRequest{
    pub start_date:DateTime<Local>,

    pub end_date:DateTime<Local>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event{
    pub all_day:bool,//false
   
    pub title:String,

    pub editable:bool,//false

    pub start_editable:bool,//false

    pub display:String,//'auto' or 'background'

    pub background_color:String,

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
  
    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let mut query=orders::table
        .left_join(merchant_members::table.on(merchant_members::member_id.nullable().eq(orders::member_id)))
        .left_join(barbers::table.on(orders::barber_id.eq(barbers::barber_id)))
        .left_join(service_types::table.on(orders::service_type_id.eq(service_types::service_type_id)))
        .filter(orders::enabled.eq(true))
        .filter(orders::merchant_id.eq(merchant_id))
        .filter(orders::end_time.ge(params.start_date).and(orders::start_time.lt(params.end_date)))
        .into_boxed();

    if let Some(barber_id)=search.barber_id{
        query=query.filter(orders::barber_id.eq(barber_id))
    }

    let data= query.order(orders::create_time.desc())
        .get_results::<(Order,Option<MerchantMember>,Option<Barber>,Option<ServiceType>)>(&mut *conn)
        .map(|v|v.into_iter().map(|t|Event{
            all_day:false,
            editable:false,
            start_editable:false,
            background_color:"rgb(56, 189, 248)".to_string(),
            display:"auto".into(),
            title: "".into(),
            extended_props:json!({
                "id":t.0.id,
                "customer": if t.0.consumer_type =="member" {
                        if t.1.as_ref().unwrap().enabled {t.1.as_ref().unwrap().real_name.clone()} else {"".into() } // 已删除
                    } else {
                        t.0.consumer_type.clone()
                    },
                "serviceName": if t.3.as_ref().unwrap().enabled {t.3.as_ref().unwrap().name.clone()} else {"".into() }, // 已删除
                "barberName":if t.2.as_ref().unwrap().enabled {t.2.as_ref().unwrap().real_name.clone()} else {"".into() }, // 已删除
            }),
            order:t.0,
        }).collect())
        .unwrap();
    
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

    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

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
        .execute(&mut *conn)
        .unwrap();

    let event=orders::table
        .left_join(merchant_members::table.on(merchant_members::member_id.nullable().eq(orders::member_id)))
        .left_join(barbers::table.on(orders::barber_id.eq(barbers::barber_id)))
        .left_join(service_types::table.on(orders::service_type_id.eq(service_types::service_type_id)))
        .filter(orders::enabled.eq(true))
        .filter(orders::merchant_id.eq(merchant_id))
        .filter(orders::order_id.eq(new_appointment.order_id))
        .get_result::<(Order,Option<MerchantMember>,Option<Barber>,Option<ServiceType>)>(&mut *conn)
        .map(|t|Event{
            all_day:false,
            editable:false,
            start_editable:false,
            background_color: "rgb(56, 189, 248)".to_string(),
            display:"auto".into(),
            title: "".into(),
            extended_props:json!({
                "id":t.0.id,
                "memberId": if t.0.consumer_type =="member" {
                        if t.1.as_ref().unwrap().enabled {t.1.as_ref().unwrap().member_id.to_string()} else {"".into() } // 已删除
                    } else {
                        "".into()
                    },
                "customer": if t.0.consumer_type =="member" {
                        if t.1.as_ref().unwrap().enabled {t.1.as_ref().unwrap().real_name.clone()} else {"".into() } // 已删除
                    } else {
                        t.0.consumer_type.clone()
                    },
                "serviceName": if t.3.as_ref().unwrap().enabled {t.3.as_ref().unwrap().name.clone()} else {"".into() }, // 已删除
                "barberName":if t.2.as_ref().unwrap().enabled {t.2.as_ref().unwrap().real_name.clone()} else {"".into() }, // 已删除
                "startTime":t.0.start_time,
                "endTime":t.0.end_time,
                "remark":t.0.remark,
                "amount":t.0.amount,
                "totalMinutes":(t.0.end_time-t.0.start_time).num_minutes(),
            }),
            order:t.0,
        })
        .unwrap();

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
    
    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let event=orders::table
        .left_join(merchant_members::table.on(merchant_members::member_id.nullable().eq(orders::member_id)))
        .left_join(barbers::table.on(orders::barber_id.eq(barbers::barber_id)))
        .left_join(service_types::table.on(orders::service_type_id.eq(service_types::service_type_id)))
        .filter(orders::enabled.eq(true))
        .filter(orders::merchant_id.eq(merchant_id))
        .filter(orders::order_id.eq(appointment_id))
        .get_result::<(Order,Option<MerchantMember>,Option<Barber>,Option<ServiceType>)>(&mut *conn)
        .map(|t|Event{
            all_day:false,
            editable:false,
            start_editable:false,
            background_color:"rgb(56, 189, 248)".to_string(),
            display:"auto".into(),
            title: "".into(),
            extended_props:json!({
                "id":t.0.id,
                "memberId": if t.0.consumer_type =="member" {
                        if t.1.as_ref().unwrap().enabled {t.1.as_ref().unwrap().member_id.to_string()} else {"".into() } // 已删除
                    } else {
                        "".into()
                    },
                "customer": if t.0.consumer_type =="member" {
                        if t.1.as_ref().unwrap().enabled {t.1.as_ref().unwrap().real_name.clone()} else {"".into() } // 已删除
                    } else {
                        t.0.consumer_type.clone()
                    },
                "serviceName": if t.3.as_ref().unwrap().enabled {t.3.as_ref().unwrap().name.clone()} else {"".into() }, // 已删除
                "barberName":if t.2.as_ref().unwrap().enabled {t.2.as_ref().unwrap().real_name.clone()} else {"".into() }, // 已删除
                "startTime":t.0.start_time,
                "endTime":t.0.end_time,
                "remark":t.0.remark,
                "amount":t.0.amount,
                "totalMinutes":(t.0.end_time-t.0.start_time).num_minutes(),
            }),
            order:t.0,
        })
        .unwrap();
        
    Ok(Json(event))
}
