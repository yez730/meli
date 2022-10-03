use axum::{http::StatusCode, Json, extract::{Query, Path, State}};
use axum_session_authentication_middleware::session::AuthSession;
use chrono::{Local, NaiveDate};
use serde::Deserialize;
use uuid::Uuid;
use crate::{
    schema::*,
    models::{Member, NewUser, NewMember, NewLoginInfo, NewPasswordLoginProvider}, authorization_policy
};
use diesel::{
    prelude::*, // for .filter
    data_types::Cents,
}; 
use crate::{models::User, axum_pg_pool::AxumPgPool};
use super::{PaginatedListRequest,PaginatedListResponse};


#[derive(Deserialize)]
pub struct MemberRequest{
    pub cellphone:String,
    pub real_name:Option<String>,
    pub gender:Option<String>,
    pub birth_day:Option<NaiveDate>,
}

pub async fn get_members(
    State(pool):State<AxumPgPool>,
    Query(params):Query<PaginatedListRequest>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<Json<PaginatedListResponse<Member>>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error

    let get_members_query=|p:&PaginatedListRequest|{
        let mut query=members::dsl::members
            .filter(members::dsl::enabled.eq(true))
            .into_boxed();
        if let Some(key)=p.key.as_ref(){
            query=query
                .filter(members::dsl::cellphone.ilike(format!("%{key}%")).or(members::dsl::real_name.ilike(format!("%{key}%"))));   
        }
        query
    };

    let count=get_members_query(&params).count().get_result(&mut *conn).map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    let data=get_members_query(&params)
        .order(members::dsl::create_time.desc())
        .limit(params.page_size)
        .offset(params.page_index*params.page_size)
        .get_results::<Member>(&mut *conn)
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
)->Result<(),(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error
    
    //添加 TODO insert data with enabled settting false, finally set to true.
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
        member_id: &Uuid::new_v4(),
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

    Ok(())
}

pub async fn delete_member(
    State(pool):State<AxumPgPool>,
    Path(id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<(),(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
    
    let mut conn=pool.pool.get().unwrap();//TODO error

    // 先不实际删除数据
    let count=diesel::update(
        members::dsl::members
        .filter(members::dsl::member_id.eq(id))
        .filter(members::dsl::enabled.eq(true))
    )
    .set((
            members::dsl::enabled.eq(false),
            members::dsl::update_time.eq(Local::now())
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

pub async fn update_member(
    State(pool):State<AxumPgPool>,
    Path(id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
    Json(req): Json<MemberRequest>
)->Result<(),(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;
   
    let mut conn=pool.pool.get().unwrap();//TODO error

    let num=diesel::update(
        members::dsl::members
        .filter(members::dsl::member_id.eq(id))
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
    Path(id):Path<Uuid>, 
    auth: AuthSession<AxumPgPool, AxumPgPool,User>,
)->Result<Json<Member>,(StatusCode,String)>{
    //检查登录
    let _=auth.identity.as_ref().ok_or((StatusCode::UNAUTHORIZED,"no login".to_string()))?;

    //检查权限
    auth.require_permissions(vec![authorization_policy::BARBER])
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"no permission".to_string()))?;

    let mut conn=pool.pool.get().unwrap();//TODO error  
    
    let member=members::dsl::members
        .filter(members::dsl::member_id.eq(id))
        .filter(members::dsl::enabled.eq(true))
        .get_result::<Member>(&mut *conn)
        .map_err(|e|(StatusCode::INTERNAL_SERVER_ERROR,e.to_string()))?;
    
    Ok(Json(member))
}
