pub mod identity;
pub mod barber;
pub mod member;
pub mod appointment;
pub mod service_type;
pub mod register;
pub mod login;
pub mod statistic;
pub mod merchant;

use serde::{Serialize, Deserialize};
use uuid::Uuid;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedListResponse<T:Serialize> {
    //分页索引，从 0 开始
    page_index:i64,

    //分页大小
    page_size:i64,

    //获取分页时原数据的元素总数量
    total_count:i64,

    data:Vec<T>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedListRequest {
    //分页索引，从 0 开始
    page_index:i64,

    //分页大小
    page_size:i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Search{
    //搜索框
    key:Option<String>,

    barber_id:Option<Uuid>,

    filter_gender:Option<String>,
}
