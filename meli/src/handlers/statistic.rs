use axum::{Json, http::StatusCode, extract::{State, Query}};
use axum_session_authentication_middleware::session::AuthSession;
use bigdecimal::BigDecimal;
use chrono::Local;
use diesel::{QueryDsl, NullableExpressionMethods, sql_types::Nullable};
use serde::Serialize;
use uuid::Uuid;
use diesel::{
    prelude::*, // for .filter
}; 
use crate::{models::{Order, User, Member, Barber, ServiceType, RechargeRecord}, authorization_policy, axum_pg::AxumPg, constant, schema::*,my_date_format};

use super::{PaginatedListResponse, PaginatedListRequest, Search};

#[derive(Serialize)]
pub struct OrderResponse{
    #[serde(rename="id")]
    pub order_id:Uuid,

    #[serde(rename="serviceName")]
    pub service_name:String,

    #[serde(rename="consumerType")]
    pub consumer_type:String,

    #[serde(rename="memberName")]
    pub member_name:String,

    #[serde(rename="memberCellphone")]
    pub member_cellphone:String,

    pub amount:BigDecimal,

    #[serde(rename="paymentType")]
    pub payment_type:String,

    #[serde(rename="barberName")]
    pub barber_name:String,

    #[serde(rename="createTime",with = "my_date_format")]
    pub create_time:chrono::DateTime<Local>,
}

pub async fn get_orders(
    State(pg):State<AxumPg>,
    Query(params):Query<PaginatedListRequest>, 
    Query(search):Query<Search>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<Json<PaginatedListResponse<OrderResponse>>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::STATISTIC]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();

    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let fn_get_query=||{
        let mut query=orders::table
            .left_join(members::table.on(members::member_id.nullable().eq(orders::member_id)))
            .left_join(barbers::table.on(orders::barber_id.eq(barbers::barber_id)))
            .left_join(service_types::table.on(orders::service_type_id.eq(service_types::service_type_id)))
            .filter(members::enabled.is_null().or(members::enabled.is_not_null().and(members::enabled.nullable().eq(true))))
            .filter(barbers::enabled.is_null().or(barbers::enabled.is_not_null().and(barbers::enabled.nullable().eq(true))))
            .filter(service_types::enabled.is_null().or(service_types::enabled.is_not_null().and(service_types::enabled.nullable().eq(true))))
            .filter(orders::enabled.eq(true))
            .filter(orders::merchant_id.eq(merchant_id))
            .into_boxed();
        
        if let Some(key)=search.key.as_ref() {
            if key.len()>0 {
                query=query.filter(members::real_name.ilike(format!("%{key}%")));   
            }
        }

        query
    };
    
    let count=fn_get_query()
        .count()
        .get_result(&mut *conn)
        .unwrap();
    let data=fn_get_query()
        .order(orders::create_time.desc())
        .limit(params.page_size)
        .offset(params.page_index*params.page_size)
        .get_results::<(Order,Option<Member>,Option<Barber>,Option<ServiceType>)>(&mut *conn)
        .map(|v|v.into_iter().map(|t|OrderResponse{
            order_id:t.0.order_id,
            service_name:t.3.map(|s|s.name).unwrap_or("-".into()),
            consumer_type: if t.0.consumer_type =="member" {
                    "会员".into()
                } else {
                    "进店顾客".into()
                },
            member_name:if t.0.consumer_type =="member" {
                    t.1.as_ref().and_then(|m|m.real_name.clone()).unwrap_or("-".into())
                } else {
                    "".into()
                },
            member_cellphone:if t.0.consumer_type =="member" {
                    t.1.as_ref().map(|m|m.cellphone.clone()).unwrap_or("-".into())
                } else {
                    "".into()
                },
            amount:t.0.amount,
            payment_type: if t.0.payment_type=="member" {"会员充值".into()} else {"现金".into()},
            barber_name:t.2.and_then(|b|b.real_name).unwrap_or("-".into()),
            create_time:t.0.create_time,
        }).collect())
        .unwrap();
    
    Ok(Json(PaginatedListResponse{
        page_index:params.page_index,
        page_size:params.page_size,
        total_count:count,
        data:data,
    }))
}

#[derive(Serialize)]
pub struct RechargeRecordResponse{
    #[serde(rename="id")]
    pub recharge_record_id:Uuid,

    #[serde(rename="memberName")]
    pub member_name:String,

    #[serde(rename="memberCellphone")]
    pub member_cellphone:String,
    
    pub amount:BigDecimal,

    #[serde(rename="barberName")]
    pub barber_name:String,

    #[serde(rename="crateTime",with = "my_date_format")]
    pub crate_time:chrono::DateTime<Local>,
}

pub async fn get_recharge_records(
    State(pg):State<AxumPg>,
    Query(params):Query<PaginatedListRequest>, 
    Query(search):Query<Search>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<Json<PaginatedListResponse<RechargeRecordResponse>>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::STATISTIC]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();

    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let fn_get_query=||{
        let mut query=recharge_records::table
        .left_join(members::table.on(recharge_records::member_id.eq(members::member_id)))
        .left_join(barbers::table.on(recharge_records::barber_id.eq(barbers::barber_id)))
        .filter(members::enabled.is_null().or(members::enabled.is_not_null().and(members::enabled.nullable().eq(true))))
        .filter(barbers::enabled.is_null().or(barbers::enabled.is_not_null().and(barbers::enabled.nullable().eq(true))))
        .filter(recharge_records::enabled.eq(true))
        .filter(recharge_records::merchant_id.eq(merchant_id))
        .into_boxed();
        
        if let Some(key)=search.key.as_ref() {
            if key.len()>0 {
                query=query.filter(members::real_name.ilike(format!("%{key}%")));   
            }
        }

        query
    };
   
    let count=fn_get_query()
        .count()
        .get_result(&mut *conn)
        .unwrap();
    let data=fn_get_query()
        .order(recharge_records::create_time.desc())
        .limit(params.page_size)
        .offset(params.page_index*params.page_size)
        .get_results::<(RechargeRecord,Option<Member>,Option<Barber>)>(&mut *conn)
        .map(|v|v.into_iter().map(|t|RechargeRecordResponse{
            recharge_record_id:t.0.recharge_record_id,
            member_name:t.1.as_ref().and_then(|m|m.real_name.clone()).unwrap_or("-".into()),
            member_cellphone:t.1.as_ref().map(|m|m.cellphone.clone()).unwrap_or("-".into()),
            amount:t.0.amount,
            barber_name:t.2.and_then(|b|b.real_name).unwrap_or("-".into()),
            crate_time:t.0.create_time,
        }).collect())
        .unwrap();
    
    Ok(Json(PaginatedListResponse{
        page_index:params.page_index,
        page_size:params.page_size,
        total_count:count,
        data:data,
    }))
}
