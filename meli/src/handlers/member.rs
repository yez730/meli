use axum::{http::StatusCode, Json, extract::{Query, Path, State}};
use axum_session_authentication_middleware::session::AuthSession;
use bigdecimal::{BigDecimal, Zero};
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{
    schema::*,
    models::{Member, NewUser, NewMember,  NewMerchantMember, MerchantMember, NewRechargeRecord, Barber, LoginInfo, NewLoginInfo}, authorization_policy, constant
};
use diesel::prelude::*; 
use crate::{models::User, axum_pg_pool::AxumPgPool};
use super::{PaginatedListRequest,PaginatedListResponse, Search};

#[derive(Deserialize)]
pub struct MemberRequest{
    pub cellphone:String,
    pub real_name:Option<String>,
    pub gender:Option<String>,
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
    
    let mut conn=pool.pool.get().unwrap();
    
    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let get_members_query=||{
        let mut query=members::table.inner_join(merchant_members::table.on(members::member_id.eq(merchant_members::member_id)))
            .filter(members::enabled.eq(true))
            .filter(merchant_members::enabled.eq(true))
            .filter(merchant_members::merchant_id.eq(merchant_id))
            .into_boxed();
            
        if let Some(key)=search.key.as_ref(){
            if key.len()>0{
                query=query.filter(members::cellphone.ilike(format!("%{key}%")).or(members::real_name.ilike(format!("%{key}%"))));  
            }
        }

        if let Some(gender)=search.filter_gender.as_ref(){
            if gender.len()>0{
                query=query.filter(members::gender.eq(gender));  
            }
        }

        query
    };

    let count=get_members_query().count().get_result(&mut *conn).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    let data=get_members_query()
        .order(members::create_time.desc())
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
    
    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let login_info=login_infos::table
        .filter(login_infos::enabled.eq(true))
        .filter(login_infos::login_info_type.eq("Cellphone"))
        .filter(login_infos::login_info_account.eq(req.cellphone.clone()))
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

            tracing::debug!("{}",format!("已存在会员 user_id: {}",existed_member.user_id));

            member=existed_member;            
        } else {
            let new_member=NewMember{
                user_id:  &login_info.user_id,
                member_id: &Uuid::new_v4(),
                cellphone:req.cellphone.as_ref(),
                real_name:req.real_name.as_deref(),
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
            .get_result::<Member>(&mut *conn).map_err(|e|{
                tracing::error!("{}",e.to_string());
                (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
            })?;
        }
    } else{
        let user_id=Uuid::new_v4();
        let new_user=NewUser{
            user_id: &user_id,
            description: "",
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
        .execute(&mut *conn).map_err(|e|{
            tracing::error!("{}",e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
        })?;            

        let new_member=NewMember{
            user_id:  &user_id,
            member_id: &Uuid::new_v4(),
            cellphone:req.cellphone.as_ref(),
            real_name:req.real_name.as_deref(),
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
        .get_result::<Member>(&mut *conn).map_err(|e|{
            tracing::error!("{}",e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
        })?;        
    }

    //TODO permissions & password login info

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
    .get_result::<MerchantMember>(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

    let member_response=MemberResponse{
        member,
        balance,
    };
     
    Ok(Json(member_response))

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

    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

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
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
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
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

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

    let member=members::table
        .filter(members::member_id.eq(member_id))
        .filter(members::enabled.eq(true))
        .get_result::<Member>(&mut *conn)
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;

    let login_info=login_infos::table
        .filter(login_infos::enabled.eq(true))
        .filter(login_infos::login_info_type.eq("Cellphone"))
        .filter(login_infos::login_info_account.eq(req.cellphone.clone()))
        .get_result::<LoginInfo>(&mut *conn)
        .ok();
    if let Some(login_info)=login_info {
        if login_info.user_id!=member.user_id{
            return Err((StatusCode::BAD_REQUEST,"该手机号已被其他会员占用".to_string()));
        } 
    } else {
        diesel::update(
            login_infos::table
            .filter(login_infos::user_id.eq(member.user_id))
            .filter(login_infos::enabled.eq(true))
        )
        .set((
            login_infos::login_info_account.eq(req.cellphone.clone()),
            login_infos::update_time.eq(Local::now())
            ))
        .execute(&mut *conn).map_err(|e|{
            tracing::error!("{}",e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
        })?;
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
            members::update_time.eq(Local::now())
        ))
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

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
    
    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let member=members::table.inner_join(merchant_members::table.on(members::member_id.eq(merchant_members::member_id)))
        .filter(members::enabled.eq(true))
        .filter(merchant_members::enabled.eq(true))
        .filter(merchant_members::member_id.eq(member_id))
        .filter(merchant_members::merchant_id.eq(merchant_id))
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
    
    let merchant_id=serde_json::from_str::<Uuid>(auth.axum_session.lock().unwrap().get_data(constant::MERCHANT_ID)).unwrap();

    let existed=members::table
        .filter(members::enabled.eq(true))
        .filter(members::member_id.eq(member_id))
        .get_result::<Member>(&mut *conn)
        .ok();

    if existed.is_none(){
        return Err((StatusCode::INTERNAL_SERVER_ERROR,"会员不存在".to_string()));
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
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;
    
    let barber=barbers::table
    .filter(barbers::enabled.eq(true))
    .filter(barbers::merchant_id.eq(merchant_id))
    .filter(barbers::user_id.eq(auth.identity.unwrap().user_id))
    .get_result::<Barber>(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

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
    .execute(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;
    
    Ok(())
}
