use axum::{http::StatusCode, Json, extract::{Query, Path, State}};
use axum_session_authentication_middleware::session::AuthSession;
use bigdecimal::{BigDecimal, Zero};
use chrono::{Local, NaiveDate};
use serde::Deserialize;
use uuid::Uuid;
use crate::{
    schema::*,
    models::*, 
    authorization_policy, 
    constant
};
use diesel::{prelude::*, select, dsl::exists}; 
use crate::{models::User, axum_pg::AxumPg};
use super::{PaginatedListRequest,PaginatedListResponse, Search, statistic::{OrderResponse, RechargeRecordResponse}};

#[derive(Deserialize)]
pub struct MemberRequest{
    pub cellphone:String,

    #[serde(rename ="realName")]
    pub real_name:String,

    pub gender:Option<String>,

    #[serde(rename ="birthDay")]
    pub birth_day:Option<NaiveDate>,

    pub remark:Option<String>,
}

pub async fn get_members(
    State(pg):State<AxumPg>,
    Query(params):Query<PaginatedListRequest>, 
    Query(search):Query<Search>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<Json<PaginatedListResponse<MerchantMember>>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();
    
    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let fn_get_members_query=||{
        let mut query=merchant_members::table
            .filter(merchant_members::enabled.eq(true))
            .filter(merchant_members::merchant_id.eq(merchant_id))
            .into_boxed();
            
        if let Some(key)=search.key.as_ref(){
            query=query.filter(merchant_members::cellphone.ilike(format!("%{key}%")).or(merchant_members::real_name.ilike(format!("%{key}%"))));  
        }

        if let Some(gender)=search.filter_gender.as_ref(){
            query=query.filter(merchant_members::gender.eq(gender));  
        }

        query
    };

    let count=fn_get_members_query()
        .count()
        .get_result(&mut *conn)
        .unwrap();
    let data=fn_get_members_query()
        .order(merchant_members::create_time.desc())
        .limit(params.page_size)
        .offset(params.page_index*params.page_size)
        .get_results::<MerchantMember>(&mut *conn)
        .unwrap();
    
    Ok(Json(PaginatedListResponse{
        page_index:params.page_index,
        page_size:params.page_size,
        total_count:count,
        data:data,
    }))
}

pub async fn add_member(
    State(pg):State<AxumPg>,
    auth: AuthSession<AxumPg, AxumPg,User>,
    Json(req): Json<MemberRequest>
)->Result<Json<MerchantMember>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();
    
    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let member_existed= select(exists(
        merchant_members::table
        .filter(merchant_members::enabled.eq(true))
        .filter(merchant_members::merchant_id.eq(merchant_id))
        .filter(merchant_members::cellphone.eq(&req.cellphone))
        ))
        .get_result::<bool>(&mut *conn)
        .unwrap();
    if member_existed {
        return Err((StatusCode::BAD_REQUEST,"已添加该手机号的理发师".to_string()));
    }

    let new_member=NewMerchantMember{
        merchant_id:&merchant_id,
        member_id: &Uuid::new_v4(),
        balance:&BigDecimal::zero(),

        enabled:true,
        create_time: Local::now(),
        update_time: Local::now(),
        data: None,

        cellphone:req.cellphone.as_ref(),
        real_name:req.real_name.as_ref(),
        gender:req.gender.as_deref(),
        birth_day:req.birth_day,
        remark:req.remark.as_deref(),
    };
    let member=diesel::insert_into(merchant_members::table)
        .values(&new_member)
        .get_result::<MerchantMember>(&mut *conn)
        .unwrap();
     
    Ok(Json(member))
}

pub async fn delete_member(
    State(pg):State<AxumPg>,
    Path(member_id):Path<Uuid>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<(),(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();

    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let member_existed = select(exists(
        merchant_members::table
        .filter(merchant_members::enabled.eq(true))
        .filter(merchant_members::merchant_id.eq(merchant_id))
        .filter(merchant_members::member_id.eq(member_id))
        ))
        .get_result::<bool>(&mut *conn)
        .unwrap();
    if !member_existed {
        return Err((StatusCode::BAD_REQUEST,"会员不存在".to_string()));
    }

    diesel::update(
        merchant_members::table
        .filter(merchant_members::member_id.eq(member_id))
        .filter(merchant_members::merchant_id.eq(merchant_id))
        .filter(merchant_members::enabled.eq(true))
    )
    .set((
        merchant_members::enabled.eq(false),
        merchant_members::update_time.eq(Local::now())
    ))
    .execute(&mut *conn)
    .unwrap();

    Ok(())
}

pub async fn update_member(
    State(pg):State<AxumPg>,
    Path(member_id):Path<Uuid>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
    Json(req): Json<MemberRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
   
    let mut conn=pg.pool.get().unwrap();

    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();
    
    let member_existed= select(exists(
        merchant_members::table
        .filter(merchant_members::enabled.eq(true))
        .filter(merchant_members::merchant_id.eq(merchant_id))
        .filter(merchant_members::member_id.eq(member_id))
        ))
        .get_result::<bool>(&mut *conn)
        .unwrap();
    if !member_existed {
        return Err((StatusCode::BAD_REQUEST,"会员不存在".to_string()));
    }

    let is_cellphone_used= select(exists(
        merchant_members::table
        .filter(merchant_members::enabled.eq(true))
        .filter(merchant_members::merchant_id.eq(merchant_id))
        .filter(merchant_members::cellphone.eq(&req.cellphone))
        .filter(merchant_members::member_id.ne(member_id))
        ))
        .get_result(&mut *conn)
        .unwrap();
    if is_cellphone_used {
        return Err((StatusCode::BAD_REQUEST,"该手机号已被使用".to_string()));
    }

    diesel::update(
        merchant_members::table
        .filter(merchant_members::member_id.eq(member_id))
        .filter(merchant_members::merchant_id.eq(merchant_id))
        .filter(merchant_members::enabled.eq(true))
    )
    .set((
        merchant_members::cellphone.eq(req.cellphone),
        merchant_members::real_name.eq(req.real_name),
        merchant_members::gender.eq(req.gender),
        merchant_members::birth_day.eq(req.birth_day),
        merchant_members::remark.eq(req.remark),
        merchant_members::update_time.eq(Local::now())
    ))
    .execute(&mut *conn)
    .unwrap();

    Ok(())
}

pub async fn get_member(
    State(pg):State<AxumPg>,
    Path(member_id):Path<Uuid>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<Json<MerchantMember>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;

    let mut conn=pg.pool.get().unwrap();
    
    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let member=merchant_members::table
        .filter(merchant_members::enabled.eq(true))
        .filter(merchant_members::member_id.eq(member_id))
        .filter(merchant_members::merchant_id.eq(merchant_id))
        .get_result::<MerchantMember>(&mut *conn)
        .map_err(|e|(StatusCode::NOT_FOUND,e.to_string()))?;
        
    Ok(Json(member))
}

#[derive(Deserialize)]
pub struct RechargeRequest{
    amount:BigDecimal,
}

pub async fn recharge(
    State(pg):State<AxumPg>,
    auth: AuthSession<AxumPg, AxumPg,User>,
    Path(member_id):Path<Uuid>, 
    Json(req): Json<RechargeRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();
    
    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let member_existed= select(exists(
        merchant_members::table
        .filter(merchant_members::enabled.eq(true))
        .filter(merchant_members::merchant_id.eq(merchant_id))
        .filter(merchant_members::member_id.eq(member_id))
        ))
        .get_result::<bool>(&mut *conn)
        .unwrap();
    if !member_existed {
        return Err((StatusCode::BAD_REQUEST,"会员不存在".to_string()));
    }
    
    if req.amount<=BigDecimal::zero() {
        return Err((StatusCode::BAD_REQUEST,"充值金额必须大于0".to_string()));
    }

    diesel::update(
        merchant_members::table
        .filter(merchant_members::member_id.eq(member_id))
        .filter(merchant_members::merchant_id.eq(merchant_id))
        .filter(merchant_members::enabled.eq(true))
    )
    .set((
        merchant_members::balance.eq(merchant_members::balance + &req.amount),
        merchant_members::update_time.eq(Local::now())
    ))
    .execute(&mut *conn)
    .unwrap();
    
    let barber=barbers::table
        .filter(barbers::enabled.eq(true))
        .filter(barbers::merchant_id.eq(merchant_id))
        .filter(barbers::user_id.eq(auth.identity.unwrap().user_id))
        .get_result::<Barber>(&mut *conn)
        .unwrap();

    let new_recharge_record=NewRechargeRecord{
        recharge_record_id:&Uuid::new_v4(),
        merchant_id:&merchant_id,
        member_id: &member_id,
        amount:&req.amount,
        barber_id:&barber.barber_id,
        enabled:true,
        create_time: Local::now(),
        update_time: Local::now(),
        data: None,
    };
    diesel::insert_into(recharge_records::table)
        .values(&new_recharge_record)
        .execute(&mut *conn)
        .unwrap();
    
    Ok(())
}

pub async fn get_orders_by_member_id(
    State(pg):State<AxumPg>,
    Query(params):Query<PaginatedListRequest>, 
    Path(member_id):Path<Uuid>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<Json<PaginatedListResponse<OrderResponse>>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::STATISTIC]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();

    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let fn_get_query=||{
        orders::table
            .inner_join(merchant_members::table.on(merchant_members::member_id.nullable().eq(orders::member_id)))
            .left_join(barbers::table.on(orders::barber_id.eq(barbers::barber_id)))
            .left_join(service_types::table.on(orders::service_type_id.eq(service_types::service_type_id)))
            .filter(merchant_members::enabled.eq(true))
            .filter(merchant_members::member_id.eq(member_id))
            .filter(orders::enabled.eq(true))
            .filter(orders::merchant_id.eq(merchant_id))
            .into_boxed()
    };
    
    let count=fn_get_query()
        .count()
        .get_result(&mut *conn)
        .unwrap();
    let data=fn_get_query()
        .order(orders::create_time.desc())
        .limit(params.page_size)
        .offset(params.page_index*params.page_size)
        .get_results::<(Order,MerchantMember,Option<Barber>,Option<ServiceType>)>(&mut *conn)
        .map(|v|v.into_iter().map(|t|OrderResponse{
            order_id:t.0.order_id,
            service_name:t.3.map(|s|s.name).unwrap_or("-".into()),
            consumer_type: if t.0.consumer_type =="member" {
                    "会员".into()
                } else {
                    "进店顾客".into()
                },
            member_name: t.1.real_name.clone(),
            member_cellphone:t.1.cellphone.clone(),
            amount:t.0.amount,
            total_minutes:(t.0.end_time-t.0.start_time).num_minutes(),
            payment_type: if t.0.payment_type=="member" {"会员充值".into()} else {"现金".into()},
            barber_name: if t.2.as_ref().unwrap().enabled {t.2.as_ref().unwrap().real_name.clone()} else {"-".into()},
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

pub async fn get_recharge_records_by_member_id(
    State(pg):State<AxumPg>,
    Query(params):Query<PaginatedListRequest>, 
    Path(member_id):Path<Uuid>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<Json<PaginatedListResponse<RechargeRecordResponse>>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::STATISTIC]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();

    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let fn_get_query=||{
        recharge_records::table
        .inner_join(merchant_members::table.on(recharge_records::member_id.eq(merchant_members::member_id)))
        .left_join(barbers::table.on(recharge_records::barber_id.eq(barbers::barber_id)))
        .filter(merchant_members::enabled.eq(true))
        .filter(merchant_members::member_id.eq(member_id))
        .filter(recharge_records::enabled.eq(true))
        .filter(recharge_records::merchant_id.eq(merchant_id))
        .into_boxed()
    };
   
    let count=fn_get_query()
        .count()
        .get_result(&mut *conn)
        .unwrap();
    let data=fn_get_query()
        .order(recharge_records::create_time.desc())
        .limit(params.page_size)
        .offset(params.page_index*params.page_size)
        .get_results::<(RechargeRecord,MerchantMember,Option<Barber>)>(&mut *conn)
        .map(|v|v.into_iter().map(|t|RechargeRecordResponse{
            recharge_record_id:t.0.recharge_record_id,
            member_name: t.1.real_name.clone(),
            member_cellphone:t.1.cellphone.clone(),
            amount:t.0.amount,
            barber_name:if t.2.as_ref().unwrap().enabled { t.2.as_ref().unwrap().real_name.clone()} else {"-".into() },
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
