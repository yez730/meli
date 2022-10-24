use std::env;

use axum::{extract::State,http::StatusCode, Json};
use axum_session_authentication_middleware::session::AuthSession;
use chrono::Local;
use serde::Deserialize;
use uuid::Uuid;

use crate::{axum_pg_pool::AxumPgPool, models::{User, NewMerchant, NewUser, NewLoginInfo, NewPasswordLoginProvider, NewBarber, Barber, Merchant, Permission, LoginInfo}, schema::{login_infos, merchants, users, password_login_providers, barbers}, authorization_policy, constant};
use diesel::{
    prelude::*,
    select, 
    dsl::exists,
};
use crate::schema::*;
use super::barber::BarberResponse; 

#[derive(Deserialize)]
pub struct RegisterMerchantRequest{
    pub merchant_name:String,
    pub login_account:String,
    pub pasword:String,
}

pub async fn register_merchant(State(pool):State<AxumPgPool>,mut auth: AuthSession<AxumPgPool, AxumPgPool,User>,Json(req):Json<RegisterMerchantRequest>)->Result<Json<BarberResponse>,(StatusCode,String)>{
    let mut conn=pool.pool.get().unwrap();
    
    let existed_merchant=select(exists(
            merchants::table
            .filter(merchants::enabled.eq(true))
            .filter(merchants::merchant_name.eq(req.merchant_name.clone()))
        ))
        .get_result(&mut *conn)
        .map_err(|_|(StatusCode::INTERNAL_SERVER_ERROR,"get_result error".to_string()))?;
    if existed_merchant{
        return Err((StatusCode::BAD_REQUEST,"该商户名已存在".to_string()));
    }

    let new_merchant=NewMerchant{
        merchant_id: &Uuid::new_v4(),
        merchant_name:req.merchant_name.as_ref(),
        company_name:None,
        credential_no:None,
        enabled:true,
        create_time: Local::now(),
        update_time: Local::now(),
        data: None,
        address:None,
        remark:None,
     };
     let merchant=diesel::insert_into(merchants::table)
     .values(&new_merchant)
     .get_result::<Merchant>(&mut *conn).map_err(|e|{
         tracing::error!("{}",e.to_string());
         (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
     })?;

    let mut user:User;

    let login_info=login_infos::table
    .filter(login_infos::enabled.eq(true))
    .filter(login_infos::login_info_account.eq(req.login_account.clone()))
    .get_result::<LoginInfo>(&mut *conn)
    .ok();

    if let Some(login_info)=login_info{
        user=users::table
        .filter(users::enabled.eq(true))
        .filter(users::user_id.eq(login_info.user_id))
        .get_result(&mut *conn).map_err(|e|{
            tracing::error!("{}",e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
        })?;
    }else {
        let user_description=format!("Administrator of merchant {}",req.merchant_name);
        let new_user=NewUser{
            user_id: &Uuid::new_v4(),
            description: user_description.as_str(),
            permissions:&serde_json::to_string(authorization_policy::DEFAULT_PERMISSIONS_OF_MERCHANT_BARBER).unwrap(),
            roles:"[]",
            enabled:true,
            create_time: Local::now(),
            update_time: Local::now(),
            data: None,
        };
        user=diesel::insert_into(users::table)
        .values(&new_user)
        .get_result::<User>(&mut *conn).map_err(|e|{
            tracing::error!("{}",e.to_string());
            (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
        })?;

        let login_info=NewLoginInfo{
            login_info_id: &Uuid::new_v4(),
            login_info_account: &req.login_account,
            login_info_type: "Cellphone", //TODO get from account by regex
            user_id: &user.user_id,
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
    }
   
     let new_barber=NewBarber{
        user_id:  &user.user_id,
        barber_id: &Uuid::new_v4(),
        merchant_id:&new_merchant.merchant_id,
        email:None,
        cellphone:&req.login_account, //TODO nullable
        real_name:None,
        enabled:true,
        create_time: Local::now(),
        update_time: Local::now(),
        data: None,
    };
    let barber=diesel::insert_into(barbers::table)
    .values(&new_barber)
    .get_result::<Barber>(&mut *conn).map_err(|e|{
        tracing::error!("{}",e.to_string());
        (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
    })?;

    // add permissions
    for &permission_code in authorization_policy::DEFAULT_PERMISSIONS_OF_MERCHANT_BARBER{
        let permissions=permissions::table
            .filter(permissions::permission_id.eq_any(serde_json::from_str::<Vec<Uuid>>(&user.permissions).unwrap())) 
            .filter(permissions::enabled.eq(true))
            .get_results::<Permission>(&mut *conn)
            .unwrap();
        let mut permission_ids=permissions.iter().map(|p|p.permission_id).collect::<Vec<_>>();
        if !permissions.into_iter().any(|p|p.permission_code==permission_code){
            let permission_id=permissions::table
            .filter(permissions::permission_code.eq(permission_code)) 
            .filter(permissions::enabled.eq(true))
            .select(permissions::permission_id)
            .get_result::<Uuid>(&mut *conn)
            .unwrap();

            permission_ids.push(permission_id);

            diesel::update(
                users::table
                .filter(users::user_id.eq(user.user_id))
                .filter(users::enabled.eq(true))
            )
            .set((
                    users::permissions.eq(serde_json::to_string(&permission_ids).unwrap()),
                    users::update_time.eq(Local::now())
                ))
            .execute(&mut *conn).map_err(|e|{
                tracing::error!("{}",e.to_string());
                (StatusCode::INTERNAL_SERVER_ERROR,e.to_string())
            })?;
        }
    }

    let salt=env::var("DATABASE_ENCRYPTION_SAULT").unwrap();
    let config = argon2::Config::default();
    let hash = argon2::hash_encoded(req.pasword.as_bytes(), salt.as_bytes(), &config).unwrap();
    let new_password_login_provider=NewPasswordLoginProvider{
        user_id: &user.user_id,
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
        
    auth.sign_in(user.user_id).await;

    auth.axum_session.lock().unwrap().set_data(constant::MERCHANT_ID.to_owned(), new_merchant.merchant_id.to_string());

    Ok(Json(BarberResponse{barber,merchant}))
}

// pub async fn register_barber(){
    
// }

// pub async fn register_member(){
    
// }
