pub const ALPHA_ADMINISTRATOR:&str="AlphaAdministrator";

//商户管理员
pub const MERCHANT_ADMINISTRATOR:&str="MerchantAdministrator";

//商户普通权限
pub const BARBER_BASE:&str="Barber_Base";
//顾客普通权限
pub const MEMBER_BASE:&str="Member_Base";

pub const CANLENDAR:&str="Canlendar";
pub const MEMBER:&str="Member";
pub const SERVICE_TYPE:&str="ServiceType";
pub const BARBER:&str="Barber";
pub const STATISTIC:&str="Statistic";

pub const DEFAULT_PERMISSIONS_OF_MERCHANT_BARBER: &'static [&'static str] = &["Canlendar", "Member", "ServiceType","Barber","Statistic"];
pub const DEFAULT_PERMISSIONS_OF_MEMBER: &'static [&'static str] = &[];//TODO Yuyue GerenXiaofeiJilu