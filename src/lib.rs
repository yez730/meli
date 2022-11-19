pub mod models;
pub mod schema;
pub mod axum_pg;
pub mod utils;
pub mod authorization_policy;
pub mod handlers;
pub mod constant;
pub mod regex_constants;

pub mod my_option_date_format {
    use chrono::{DateTime, Local, TimeZone};
    use serde::{self, Deserialize, Serializer, Deserializer};

    // const FORMAT: &str = "%+";
    const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

    pub fn serialize<S>(
        date: &Option<DateTime<Local>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match date {
            Some(value) => serializer.serialize_some(&format!("{}",value.format(FORMAT))),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Local>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let option:Option<String>=Option::deserialize(deserializer)?;
        option.map(|s|Local
            .datetime_from_str(&s, FORMAT)
            .map_err(serde::de::Error::custom)
        )
        .transpose()
    }
}

pub mod my_date_format {
    use chrono::{DateTime, TimeZone, Local};
    use serde::{self, Deserialize, Serializer, Deserializer};

    const FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

    pub fn serialize<S>(
        date: &DateTime<Local>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<DateTime<Local>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Local.datetime_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod test{
    use serde::{Serialize,Deserialize};
    use chrono::{DateTime, NaiveDate, Local};
    use super::my_option_date_format;

    #[derive(Deserialize,Serialize)]
    struct Input{
        a:String,
        b:Option<String>,

        #[serde(default, with = "my_option_date_format")]
        c:Option<DateTime<Local>>,
    }
    #[test]
    #[ignore]
    fn test_serde(){
        // use chrono::{DateTime, TimeZone, NaiveDateTime, Local};
        // let local1=Local.ymd(2015, 5, 15);
        // println!("{}",local1); //2015-05-15+08:00

        // let input1=Input{a:"123".into(),b:None,c:None};
        // let json1=serde_json::to_string(&input1).unwrap();
        // assert_eq!(json1,"");

        let input2=Input{a:"123".into(),b:Some("456".into()),c:Some(Local::now())};
        let json2=serde_json::to_string(&input2).unwrap();
        assert_eq!(json2,"");
        // my_date_format::serialize(date, serializer)

        // let json3=r#"{"a":"123","b":"456","c":"2022-09-19 22:51:32"}"#;
        // let input3=serde_json::from_str::<Input>(json3).unwrap();
        // assert_eq!(input3.a,"123");
        // assert_eq!(input3.b,Some("456".into()));
        // assert_eq!(input3.c,Some(Local::now()));

        // let json4=r#"{"a":"123","b":null}"#;
        // let json4=r#"{"a":"123"}"#;
        // let input4=serde_json::from_str::<Input>(json4).unwrap();
        // assert_eq!(input4.a,"123");
        // assert_eq!(input4.b,None);
    }

    #[derive(Deserialize,Serialize)]
    struct Input3{
        a:String,
        b:Option<NaiveDate>,
    }

    #[test]
    #[ignore]
    fn test3(){
        let h:std::collections::HashMap<String,String>=std::collections::HashMap::new();
        let s=serde_json::to_string(&h).unwrap();
        assert_eq!(s,"s");
        // let input1=Input3{a:"123".into(),b:Some(NaiveDate::from_ymd(2022, 4, 18))};
        // let json1=serde_json::to_string(&input1).unwrap(); //2022-04-18
        // assert_eq!(json1,"");

        // let json1=r#"{"a":"123","b":"2022-04-18"}"#;//serde_json::to_string(&input1).unwrap(); //2022-04-18
        // let input1=serde_json::from_str::<Input3>(json1).unwrap();
        // assert_eq!(input1.b,Some(NaiveDate::from_ymd(2022, 4, 20)));

    }
}
