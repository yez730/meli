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
pub struct PaginatedListResponse<T:Serialize> {
    //分页索引，从 0 开始
    #[serde(rename ="pageIndex")]
    page_index:i64,

    //分页大小
    #[serde(rename ="pageSize")]
    page_size:i64,

    //获取分页时原数据的元素总数量
    #[serde(rename ="totalCount")]
    total_count:i64,

    data:Vec<T>,
}

#[derive(Deserialize)]
pub struct PaginatedListRequest {
    //分页索引，从 0 开始
    #[serde(rename ="pageIndex")]
    page_index:i64,

    #[serde(rename ="pageSize")]
    //分页大小
    page_size:i64,
}

#[derive(Deserialize)]
pub struct Search{
    //搜索框
    key:Option<String>,

    #[serde(rename ="barberId")]
    barber_id:Option<Uuid>,

    #[serde(rename ="filterGender")]
    filter_gender:Option<String>,
}
