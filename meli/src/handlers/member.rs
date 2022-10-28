use axum::{http::StatusCode, Json, extract::{Query, Path, State}};
use axum_session_authentication_middleware::session::AuthSession;
use bigdecimal::{BigDecimal, Zero};
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    schema::*,
    models::{Member, NewUser, NewMember,  NewMerchantMember, MerchantMember, NewRechargeRecord, Barber, LoginInfo, NewLoginInfo, ServiceType, RechargeRecord, Order}, authorization_policy, constant
};
use diesel::prelude::*; 
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

#[derive(Serialize)]
pub struct MemberResponse{
    #[serde(flatten)]
    pub member:Member,

    #[serde(flatten)]
    pub balance:MerchantMember,
}

pub async fn get_members(
    State(pg):State<AxumPg>,
    Query(params):Query<PaginatedListRequest>, 
    Query(search):Query<Search>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<Json<PaginatedListResponse<MemberResponse>>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();
    
    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let fn_get_members_query=||{
        let mut query=members::table.inner_join(merchant_members::table.on(members::member_id.eq(merchant_members::member_id)))
            .filter(members::enabled.eq(true))
            .filter(merchant_members::enabled.eq(true))
            .filter(merchant_members::merchant_id.eq(merchant_id))
            .into_boxed();
            
        if let Some(key)=search.key.as_ref(){
            query=query.filter(members::cellphone.ilike(format!("%{key}%")).or(members::real_name.ilike(format!("%{key}%"))));  
        }

        if let Some(gender)=search.filter_gender.as_ref(){
            query=query.filter(members::gender.eq(gender));  
        }

        query
    };

    let count=fn_get_members_query()
        .count()
        .get_result(&mut *conn)
        .unwrap();
    let data=fn_get_members_query()
        .order(members::create_time.desc())
        .limit(params.page_size)
        .offset(params.page_index*params.page_size)
        .get_results::<(Member, MerchantMember)>(&mut *conn)
        .map(|v|v.into_iter().map(|(m,b)|MemberResponse { member: m, balance: b }).collect())
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
)->Result<Json<MemberResponse>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;
    
    let mut conn=pg.pool.get().unwrap();
    
    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let login_info=login_infos::table
        .filter(login_infos::enabled.eq(true))
        .filter(login_infos::login_info_type.eq("Cellphone"))
        .filter(login_infos::login_info_account.eq(&req.cellphone))
        .get_result::<LoginInfo>(&mut *conn)
        .ok();

    let member;

    if let Some(login_info)=login_info {
        let existed_member=members::table 
            .filter(members::enabled.eq(true))
            .filter(members::user_id.eq(login_info.user_id))
            .get_result::<Member>(&mut *conn)
            .ok();
        if let Some(existed_member)=existed_member{
            let merchant_member=merchant_members::table
                .filter(merchant_members::enabled.eq(true))
                .filter(merchant_members::merchant_id.eq(merchant_id))
                .filter(merchant_members::member_id.eq(existed_member.member_id))
                .get_result::<MerchantMember>(&mut *conn)
                .ok();
            if merchant_member.is_some(){
                return Err((StatusCode::BAD_REQUEST,"当前商户下已存在该手机号码的会员".to_string()));
            }

            //TODO update member 商户不允许

            tracing::warn!("{}",format!("已存在会员 user_id: {}",existed_member.user_id));

            member=existed_member;        
        } else {
            let new_member=NewMember{
                user_id:  &login_info.user_id,
                member_id: &Uuid::new_v4(),
                cellphone:req.cellphone.as_ref(),
                real_name:req.real_name.as_ref(),
                gender:req.gender.as_deref(),
                birth_day:req.birth_day,
                enabled:true,
                create_time: Local::now(),
                update_time: Local::now(),
                data: None,
                remark:None,
            };
            member=diesel::insert_into(members::table)
                .values(&new_member)
                .get_result::<Member>(&mut *conn)
                .unwrap();
        }
    } else{
        let user_id=Uuid::new_v4();
        let new_user=NewUser{
            user_id: &user_id,
            description: "后台添加",
            permissions:"[]",
            roles:"[]",
            enabled:true,
            create_time: Local::now(),
            update_time: Local::now(),
            data: None,
        };
        diesel::insert_into(users::table)
            .values(&new_user)
            .execute(&mut *conn)
            .unwrap();

        let login_info=NewLoginInfo{
            login_info_id: &Uuid::new_v4(),
            login_info_account: req.cellphone.as_ref(),
            login_info_type: "Cellphone",
            user_id: &user_id,
            enabled: true, 
            create_time: Local::now(),
            update_time: Local::now(),
        };
        diesel::insert_into(login_infos::table)
            .values(&login_info)
            .execute(&mut *conn)
            .unwrap();            

        let new_member=NewMember{
            user_id:  &user_id,
            member_id: &Uuid::new_v4(),
            cellphone:req.cellphone.as_ref(),
            real_name:req.real_name.as_ref(),
            gender:req.gender.as_deref(),
            birth_day:req.birth_day,
            enabled:true,
            create_time: Local::now(),
            update_time: Local::now(),
            data: None,
            remark:req.remark.as_deref(),
        };
        member=diesel::insert_into(members::table)
            .values(&new_member)
            .get_result::<Member>(&mut *conn)
            .unwrap();        
    }

    //TODO add permissions

    let new_balance=NewMerchantMember{
        merchant_id:&merchant_id,
        member_id: &member.member_id,
        balance:&BigDecimal::zero(),

        enabled:true,
        create_time: Local::now(),
        update_time: Local::now(),
        data: None,
    };
    let balance=diesel::insert_into(merchant_members::table)
        .values(&new_balance)
        .get_result::<MerchantMember>(&mut *conn)
        .unwrap();

    let member_response=MemberResponse{
        member,
        balance,
    };
     
    Ok(Json(member_response))
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

    let _existed=members::table
        .filter(members::enabled.eq(true))
        .filter(members::member_id.eq(member_id))
        .get_result::<Member>(&mut *conn)
        .map_err(|_|{
            (StatusCode::NOT_FOUND,"会员不存在".to_string())
        })?;

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

    let member=members::table
        .filter(members::member_id.eq(member_id))
        .filter(members::enabled.eq(true))
        .get_result::<Member>(&mut *conn)
        .map_err(|_|{
            (StatusCode::NOT_FOUND,"会员不存在".to_string())
        })?;

    let login_info=login_infos::table
        .filter(login_infos::enabled.eq(true))
        .filter(login_infos::login_info_type.eq("Cellphone"))
        .filter(login_infos::login_info_account.eq(&req.cellphone))
        .get_result::<LoginInfo>(&mut *conn)
        .ok();
    if let Some(login_info)=login_info {
        if login_info.user_id!=member.user_id{
            return Err((StatusCode::BAD_REQUEST,"该手机号已被其他会员占用，请联系管理员".to_string()));
        } 
    } else {
        diesel::update(
            login_infos::table
            .filter(login_infos::user_id.eq(member.user_id))
            .filter(login_infos::enabled.eq(true))
        )
        .set((
            login_infos::login_info_account.eq(&req.cellphone),
            login_infos::update_time.eq(Local::now())
            ))
        .execute(&mut *conn)
        .unwrap();
    }

    diesel::update(
        members::table
        .filter(members::member_id.eq(member_id))
        .filter(members::enabled.eq(true))
    )
    .set((
        members::cellphone.eq(req.cellphone),
        members::real_name.eq(req.real_name),
        members::gender.eq(req.gender),
        members::birth_day.eq(req.birth_day),
        members::remark.eq(req.remark),
        members::update_time.eq(Local::now())
    ))
    .execute(&mut *conn)
    .unwrap();

    Ok(())
}

pub async fn get_member(
    State(pg):State<AxumPg>,
    Path(member_id):Path<Uuid>, 
    auth: AuthSession<AxumPg, AxumPg,User>,
)->Result<Json<MemberResponse>,(StatusCode,String)>{
    //检查登录&权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE]).map_err(|e|(StatusCode::UNAUTHORIZED,e.to_string()))?;

    let mut conn=pg.pool.get().unwrap();
    
    let merchant_id=Uuid::parse_str(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let member=members::table.inner_join(merchant_members::table.on(members::member_id.eq(merchant_members::member_id)))
        .filter(members::enabled.eq(true))
        .filter(merchant_members::enabled.eq(true))
        .filter(merchant_members::member_id.eq(member_id))
        .filter(merchant_members::merchant_id.eq(merchant_id))
        .get_result::<(Member, MerchantMember)>(&mut *conn)
        .map(|(m,b)|MemberResponse { member: m, balance: b })
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

    let _existed=members::table
        .filter(members::enabled.eq(true))
        .filter(members::member_id.eq(member_id))
        .get_result::<Member>(&mut *conn)
        .map_err(|_|{
            (StatusCode::NOT_FOUND,"会员不存在".to_string())
        })?;
    
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
            .left_join(members::table.on(members::member_id.nullable().eq(orders::member_id)))
            .left_join(barbers::table.on(orders::barber_id.eq(barbers::barber_id)))
            .left_join(service_types::table.on(orders::service_type_id.eq(service_types::service_type_id)))
            .filter(members::enabled.eq(true))
            .filter(members::member_id.eq(member_id))
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
        .get_results::<(Order,Option<Member>,Option<Barber>,Option<ServiceType>)>(&mut *conn)
        .map(|v|v.into_iter().map(|t|OrderResponse{
            order_id:t.0.order_id,
            service_name:t.3.map(|s|s.name).unwrap_or("-".into()),
            consumer_type: if t.0.consumer_type =="member" {
                    "会员".into()
                } else {
                    "进店顾客".into()
                },
            member_name: if t.0.consumer_type =="member" {
                if t.1.as_ref().unwrap().enabled {t.1.as_ref().unwrap().real_name.clone() } else {"-".into() }
            } else {
                "".into()
            },
            member_cellphone:if t.0.consumer_type =="member" {
                if t.1.as_ref().unwrap().enabled {t.1.as_ref().unwrap().cellphone.clone() } else {"-".into() }
            } else {
                "".into()
            },
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
        .left_join(members::table.on(recharge_records::member_id.eq(members::member_id)))
        .left_join(barbers::table.on(recharge_records::barber_id.eq(barbers::barber_id)))
        .filter(members::enabled.eq(true))
        .filter(members::member_id.eq(member_id))
        .filter(barbers::enabled.is_null().or(barbers::enabled.is_not_null().and(barbers::enabled.nullable().eq(true))))
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
        .get_results::<(RechargeRecord,Option<Member>,Option<Barber>)>(&mut *conn)
        .map(|v|v.into_iter().map(|t|RechargeRecordResponse{
            recharge_record_id:t.0.recharge_record_id,
            member_name: if t.1.as_ref().unwrap().enabled { t.1.as_ref().unwrap().real_name.clone()} else {"-".into() },
            member_cellphone:if t.1.as_ref().unwrap().enabled { t.1.as_ref().unwrap().cellphone.clone()} else {"-".into() },
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
