use axum::{Json, http::StatusCode, extract::{State, Query}};
use axum_session_authentication_middleware::session::AuthSession;
use diesel::{QueryDsl, NullableExpressionMethods};
use serde::Serialize;
use uuid::Uuid;
use diesel::{
    prelude::*, // for .filter
}; 
use crate::{models::{Order, User, Member, Barber, ServiceType, RechargeRecord}, authorization_policy, axum_pg::AxumPg, constant, schema::*};

use super::{PaginatedListResponse, PaginatedListRequest, Search};

#[derive(Serialize)]
pub struct OrderResponse{
    #[serde(flatten)]
    pub member:Option<Member>,

    #[serde(flatten)]
    pub barber:Barber,

    #[serde(flatten)]
    pub service_type:ServiceType,

    #[serde(flatten)]
    pub order:Order,
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

    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let fn_get_query=||{
        let mut query=orders::table
            .left_join(members::table.on(members::member_id.nullable().eq(orders::member_id)))
            .inner_join(barbers::table.on(orders::barber_id.eq(barbers::barber_id)))
            .inner_join(service_types::table.on(orders::service_type_id.eq(service_types::service_type_id)))
            .filter(members::enabled.eq(true))
            .filter(barbers::enabled.eq(true))
            .filter(service_types::enabled.eq(true))
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
   
    let count=fn_get_query().count().get_result(&mut *conn).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    let data=fn_get_query()
        .order(orders::create_time.desc())
        .limit(params.page_size)
        .offset(params.page_index*params.page_size)
        .get_results::<(Order,Option<Member>,Barber,ServiceType)>(&mut *conn)
        .map(|v|v.into_iter().map(|t|OrderResponse{
            order:t.0,
            member:t.1,
            barber:t.2,
            service_type:t.3,
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
    #[serde(flatten)]
    pub member:Member,

    #[serde(flatten)]
    pub barber:Barber,

    #[serde(flatten)]
    pub recharge_record:RechargeRecord,
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

    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let fn_get_query=||{
        let mut query=recharge_records::table
        .inner_join(members::table.on(recharge_records::member_id.eq(members::member_id)))
        .inner_join(barbers::table.on(recharge_records::barber_id.eq(barbers::barber_id)))
        .filter(members::enabled.eq(true))
        .filter(barbers::enabled.eq(true))
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
   
    let count=fn_get_query().count().get_result(&mut *conn).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    let data=fn_get_query()
        .order(recharge_records::create_time.desc())
        .limit(params.page_size)
        .offset(params.page_index*params.page_size)
        .get_results::<(RechargeRecord,Member,Barber)>(&mut *conn)
        .map(|v|v.into_iter().map(|t|RechargeRecordResponse{
            member:t.1,
            barber:t.2,
            recharge_record:t.0,
        }).collect())
        .unwrap();
    
    Ok(Json(PaginatedListResponse{
        page_index:params.page_index,
        page_size:params.page_size,
        total_count:count,
        data:data,
    }))
}
