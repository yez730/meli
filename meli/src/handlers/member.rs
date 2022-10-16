use axum::{http::StatusCode, Json, extract::{Query, Path, State}};
use axum_session_authentication_middleware::session::AuthSession;
use bigdecimal::{BigDecimal, Zero};
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    schema::*,
    models::{Member, NewUser, NewMember, NewLoginInfo, NewPasswordLoginProvider, NewMerchantMember, MerchantMember, NewRechargeRecord, Barber}, authorization_policy
};
use diesel::{
    prelude::*, // for .filter
    select, 
    dsl::exists,
}; 
use crate::{models::User, axum_pg_pool::AxumPgPool};
use super::{PaginatedListRequest,PaginatedListResponse, Search};

#[derive(Deserialize)]
pub struct MemberRequest{
    pub cellphone:String,
    pub real_name:Option<String>,
    pub gender:Option<String>,
    pub birth_day:Option<NaiveDate>,
}

#[derive(Serialize)]
pub struct MemberResponse{
    #[serde(flatten)]
    pub member:Member,
    #[serde(flatten)]
    pub balance:MerchantMember,
}

pub async fn get_members(
    State(pool):State<AxumPgPool>,
    Query(params):Query<PaginatedListRequest>, 
    Query(search):Query<Search>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<Json<PaginatedListResponse<MemberResponse>>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error

    let barber=serde_json::from_str::<Option<Barber>>(auth.axum_session.lock().unwrap().get_data("barber"))
	    .unwrap().unwrap();

    let get_members_query=|p:&PaginatedListRequest|{
        let mut query=members::dsl::members.inner_join(merchant_members::dsl::merchant_members.on(members::dsl::member_id.eq(merchant_members::dsl::member_id)))
            .filter(members::dsl::enabled.eq(true))
            .filter(merchant_members::dsl::enabled.eq(true))
            .filter(merchant_members::dsl::merchant_id.eq(barber.merchant_id))
            .into_boxed();
            
        if let Some(key)=search.key.as_ref(){
            if key.len()>0{
                query=query.filter(members::dsl::cellphone.ilike(format!("%{key}%")).or(members::dsl::real_name.ilike(format!("%{key}%"))));  
            }
        }

        if let Some(gender)=search.filter_gender.as_ref(){
            if gender.len()>0{
                query=query.filter(members::dsl::gender.eq(gender));  
            }
        }

        query
    };

    let count=get_members_query(&params).count().get_result(&mut *conn).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    let data=get_members_query(&params)
        .order(members::dsl::create_time.desc())
        .limit(params.page_size)
        .offset(params.page_index*params.page_size)
        .get_results::<(Member, MerchantMember)>(&mut *conn)
        .map(|v|v.into_iter().map(|(m,b)|MemberResponse { member: m, balance: b }).collect())
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    
    Ok(Json(PaginatedListResponse{
        page_index:params.page_index,
        page_size:params.page_size,
        total_count:count,
        data:data,
    }))
}

pub async fn add_member(
    State(pool):State<AxumPgPool>,
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
    Json(req): Json<MemberRequest>
)->Result<Json<MemberResponse>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error
    
    let barber=serde_json::from_str::<Option<Barber>>(auth.axum_session.lock().unwrap().get_data("barber"))
	    .unwrap().unwrap();

    //添加 TODO insert data with enabled settting false, finally set to true.
    let existed=members::dsl::members
        .filter(members::dsl::enabled.eq(true))
        .filter(members::dsl::cellphone.eq(&req.cellphone))
        .get_result::<Member>(&mut *conn)
        .ok();

    let mut member_id=Uuid::new_v4();
    if let Some(member)=existed{
        member_id=member.member_id;

        let exist_member=select(exists(
            merchant_members::dsl::merchant_members
            .filter(merchant_members::dsl::enabled.eq(true))
            .filter(merchant_members::dsl::member_id.eq(member.member_id))
            .filter(merchant_members::dsl::merchant_id.eq(&barber.merchant_id))
        ))
        .get_result(&mut *conn)
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"get_result error".to_string()))?;

        if exist_member{
            return Err((StatusCode::INTERNAL_SERVER_ERROR,"已存在该用户".to_string()));
        }
        // TODO update member info ?

        let new_balance=NewMerchantMember{
            merchant_id:&barber.merchant_id,
            member_id: &member.member_id,
            balance:&BigDecimal::zero(),

            enabled:true,
            create_time: Local::now(),
            update_time: Local::now(),
            data: None,
        };

        diesel::insert_into(merchant_members::table)
        .values(&new_balance)
        .execute(&mut *conn).map_err(|e|{
            tracing::error!("{}",e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
        })?;
    } else {
        // 1. add user
        let user_id=Uuid::new_v4();
        let new_user=NewUser{
            user_id: &user_id,
            description: "test user",
            permissions:&serde_json::to_string(authorization_policy::DEFAULT_PERMISSIONS_OF_MEMBER).unwrap(),
            roles:"[]",
            enabled:true,
            create_time: Local::now(),
            update_time: Local::now(),
            data: None,
        };
        diesel::insert_into(users::table)
        .values(&new_user)
        .execute(&mut *conn).map_err(|e|{
            tracing::error!("{}",e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
        })?;

        // 2. add login info / login info provider  //TODO cellphone login info provider
        // 2.1
        let login_info=NewLoginInfo{
            login_info_id: &Uuid::new_v4(),
            login_info_barber: &req.cellphone,
            login_info_type: "Username", //TODO get enum variant value string
            user_id: &user_id,
            enabled: true, // TODO false
            create_time: Local::now(),
            update_time: Local::now(),
        };
        diesel::insert_into(login_infos::table)
        .values(&login_info)
        .execute(&mut *conn).map_err(|e|{
            tracing::error!("{}",e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
        })?;
        // 2.2
        let password = b"123456";
        let salt = b"randomsalt";
        let config = argon2::Config::default();
        let hash = argon2::hash_encoded(password, salt, &config).unwrap();
        let new_password_login_provider=NewPasswordLoginProvider{
            user_id: &user_id,
            password_hash: &hash,
            enabled:true,
            create_time: Local::now(),
            update_time: Local::now(),
            data:None
        };
        diesel::insert_into(password_login_providers::table)
        .values(&new_password_login_provider)
        .execute(&mut *conn).map_err(|e|{
            tracing::error!("{}",e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
        })?;

        // 3. add member.
        let new_member=NewMember{
            user_id:  &user_id,
            member_id: &member_id,
            cellphone:&req.cellphone,
            real_name:req.real_name.as_deref(),
            gender:req.gender.as_deref(),
            birth_day:req.birth_day,
            enabled:true,
            create_time: Local::now(),
            update_time: Local::now(),
            data: None,
        };
        diesel::insert_into(members::table)
        .values(&new_member)
        .execute(&mut *conn).map_err(|e|{
            tracing::error!("{}",e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
        })?;

        // 4. add relationship & balance.
        let new_balance=NewMerchantMember{
            merchant_id:&barber.merchant_id,
            member_id: &new_member.member_id,
            balance:&BigDecimal::zero(),

            enabled:true,
            create_time: Local::now(),
            update_time: Local::now(),
            data: None,
        };
        diesel::insert_into(merchant_members::table)
        .values(&new_balance)
        .execute(&mut *conn).map_err(|e|{
            tracing::error!("{}",e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
        })?;
    }
    
    let member=members::dsl::members.inner_join(merchant_members::dsl::merchant_members.on(members::dsl::member_id.eq(merchant_members::dsl::member_id)))
    .filter(members::dsl::enabled.eq(true))
    .filter(merchant_members::dsl::enabled.eq(true))
    .filter(merchant_members::dsl::member_id.eq(member_id))
    .filter(merchant_members::dsl::merchant_id.eq(barber.merchant_id))
    .get_result::<(Member, MerchantMember)>(&mut *conn)
    .map(|(m,b)|MemberResponse { member: m, balance: b })
    .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    Ok(Json(member))

}

pub async fn delete_member(
    State(pool):State<AxumPgPool>,
    Path(member_id):Path<Uuid>, 
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
        merchant_members::dsl::merchant_members
        .filter(merchant_members::dsl::member_id.eq(member_id))
        .filter(merchant_members::dsl::merchant_id.eq(barber.merchant_id))
        .filter(merchant_members::dsl::enabled.eq(true))
    )
    .set((
            merchant_members::dsl::enabled.eq(false),
            merchant_members::dsl::update_time.eq(Local::now())
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

// TODO 不允许商家
pub async fn update_member(
    State(pool):State<AxumPgPool>,
    Path(member_id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
    Json(req): Json<MemberRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
   
    let mut conn=pool.pool.get().unwrap();//TODO error

    // let barber=serde_json::from_str::<Option<Barber>>(auth.axum_session.lock().unwrap().get_data("barber"))
    // 	.unwrap().unwrap();

    let num=diesel::update(
        members::dsl::members
        .filter(members::dsl::member_id.eq(member_id))
        .filter(members::dsl::enabled.eq(true))
    )
    .set((
            members::dsl::cellphone.eq(req.cellphone),
            members::dsl::real_name.eq(req.real_name),
            members::dsl::gender.eq(req.gender),
            members::dsl::birth_day.eq(req.birth_day),
            members::dsl::update_time.eq(Local::now())
        ))
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

    if num !=1 {
        tracing::error!("update_member affected num: {}",num);
    }
    
    Ok(())
}

pub async fn get_member(
    State(pool):State<AxumPgPool>,
    Path(member_id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<Json<MemberResponse>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;

    let mut conn=pool.pool.get().unwrap();//TODO error  
    
    let barber=serde_json::from_str::<Option<Barber>>(auth.axum_session.lock().unwrap().get_data("barber"))
	    .unwrap().unwrap();

    let member=members::dsl::members.inner_join(merchant_members::dsl::merchant_members.on(members::dsl::member_id.eq(merchant_members::dsl::member_id)))
        .filter(members::dsl::enabled.eq(true))
        .filter(merchant_members::dsl::enabled.eq(true))
        .filter(merchant_members::dsl::member_id.eq(member_id))
        .filter(merchant_members::dsl::merchant_id.eq(barber.merchant_id))
        .get_result::<(Member, MerchantMember)>(&mut *conn)
        .map(|(m,b)|MemberResponse { member: m, balance: b })
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
        
    Ok(Json(member))
}

#[derive(Deserialize)]
pub struct RechargeRequest{
    amount:BigDecimal,
}

pub async fn recharge(
    State(pool):State<AxumPgPool>,
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
    Path(member_id):Path<Uuid>, 
    Json(req): Json<RechargeRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER_BASE])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error
    
    let barber=serde_json::from_str::<Option<Barber>>(auth.axum_session.lock().unwrap().get_data("barber"))
	    .unwrap().unwrap();

    let existed=members::dsl::members
        .filter(members::dsl::enabled.eq(true))
        .filter(members::dsl::member_id.eq(member_id))
        .get_result::<Member>(&mut *conn)
        .ok();

    if existed.is_none(){
        return Err((StatusCode::INTERNAL_SERVER_ERROR,"会员不存在".to_string()));
    }

    let num=diesel::update(
        merchant_members::dsl::merchant_members
        .filter(merchant_members::dsl::member_id.eq(member_id))
        .filter(merchant_members::dsl::merchant_id.eq(barber.merchant_id))
        .filter(merchant_members::dsl::enabled.eq(true))
    )
    .set((
            merchant_members::dsl::balance.eq(merchant_members::dsl::balance + &req.amount),
            merchant_members::dsl::update_time.eq(Local::now())
        ))
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

    if num !=1 {
        tracing::error!("update_member affected num: {}",num);
    }

    let new_recharge_record=NewRechargeRecord{
        recharge_record_id:&Uuid::new_v4(),
        merchant_id:&barber.merchant_id,
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
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;
    
    Ok(())
}
