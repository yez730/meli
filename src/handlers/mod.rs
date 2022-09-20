pub mod user_handler;

use serde::{Serialize, Deserialize};

#[derive(Serialize)]
pub struct PaginatedListResponse<T:Serialize> {
    //分页索引，从 0 开始
    page_index:i32,

    //分页大小
    page_size:i32,

    //获取分页时原数据的元素总数量
    total_count:i32,

    //获取分页时原数据的元素总页数。
    total_page_count:i32,// = (int)Math.Ceiling(totalCount / (double)pageSize);

    data:Vec<T>,
}

#[derive(Deserialize)]
pub struct PaginatedListRequest {
    //分页索引，从 0 开始
    page_index:i32,

    //分页大小
    page_size:i32,

    //搜索框
    key:Option<String>,
}
