pub mod identity;
pub mod barber;
pub mod member;
pub mod order;
pub mod service_type;

use serde::{Serialize, Deserialize};

#[derive(Serialize)]
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
pub struct PaginatedListRequest {
    //分页索引，从 0 开始
    page_index:i64,

    //分页大小
    page_size:i64,

    //搜索框
    key:Option<String>,
}
