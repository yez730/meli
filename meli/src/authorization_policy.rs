pub const ALPHA_ADMINISTRATOR:&str="AlphaAdministrator";

//商户管理员
pub const MERCHANT_ADMINISTRATOR:&str="MerchantAdministrator";

//商户普通权限
pub const BARBER:&str="Barber";
//顾客普通权限
pub const MEMBER:&str="Member";

pub const SHOUYE:&str="Shouye";
pub const QIANTAI_YINGYE:&str="QiantaiYingye";
pub const KEHU_GUANLI:&str="KehuGuanli";
pub const YEWU_TONGJI:&str="YewuTongji";
pub const HOUTAI_GUANLI:&str="HoutaiGuanli";

pub const DEFAULT_PERMISSIONS_OF_MERCHANT_BARBER: &'static [&'static str] = &["Shouye", "QiantaiYingye", "KehuGuanli","YewuTongji","HoutaiGuanli"];
pub const DEFAULT_PERMISSIONS_OF_MEMBER: &'static [&'static str] = &[];//TODO Yuyue GerenXiaofeiJilu