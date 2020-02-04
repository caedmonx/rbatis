use std::borrow::{Borrow, BorrowMut};
use std::cell::RefMut;
use std::collections::LinkedList;
use std::fs;
use std::ops::Deref;
use std::process::exit;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::mpsc;
use std::sync::Mutex;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

use actix_web::{App, HttpServer, Responder, web};
use log::{error, info, warn};
use rdbc::{DataType, Driver, ResultSet, ResultSetMetaData};
use serde_json::{json, Number, Value};


use crate::ast::node::bind_node::BindNode;
use crate::ast::node::node_type::NodeType;
use crate::crud::ipage::IPage;
use crate::decode::encoder::encode_to_value;
use crate::decode::rdbc_driver_decoder;
use crate::decode::rdbc_driver_decoder::decode_result_set;
use crate::example::activity::Activity;
use crate::example::conf::MYSQL_URL;
use crate::rbatis::{eval_sql, Rbatis, singleton};
use crate::session_factory::{SessionFactory, SessionFactoryCached};
use crate::tx::propagation::Propagation::{NONE, REQUIRED};
use crate::tx::propagation::Propagation;
use crate::error::RbatisError;

/**
 初始化实例
*/
fn init_rbatis() -> Result<Rbatis, RbatisError> {
    //1 启用日志(可选，不添加则不加载日志库)
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();

    let mut rbatis = Rbatis::new();

    //3 加载数据库url name 为空，则默认数据库
    rbatis.load_db_url("", MYSQL_URL);//"mysql://root:TEST@localhost:3306/test"
    //4 加载xml配置

    let f = fs::File::open("./src/example/Example_ActivityMapper.xml");
    rbatis.load_xml("Example_ActivityMapper.xml".to_string(), fs::read_to_string("./src/example/Example_ActivityMapper.xml").unwrap());//加载xml数据
    //判断是否配置数据库
    let conf = rbatis.db_driver_map.get("").unwrap();
    if conf.contains("localhost") {
        error!("{}", "请修改mysql链接'mysql://root:TEST@localhost:3306/test' 替换为具体的 用户名，密码，ip，和数据库名称");
        return Err(RbatisError::from("请修改mysql链接'mysql://root:TEST@localhost:3306/test' 替换为具体的 用户名，密码，ip，和数据库名称".to_string()));
    }

//    自定义动态数据源路由return 的字符串为 rbatis.db_router 中定义的配置的key(默认""为默认配置)（在此之前需要加载配置rbatis.load_db_url()）
//    rbatis.router_func = |id| -> String{
//        info!("匹配路由key  ====>  {}",id);
//        //例如：你可以自定义读写分离
//        if id.contains("select"){
//            //info!("select开头 加载读路由配置");
//        }else{
//            //info!("非select开头 加载写路由配置");
//        }
//        return "".to_string();
//    };
    return Ok(rbatis);
}

fn init_singleton_rbatis() {
    //1 启用日志(可选，不添加则不加载日志库)
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    //3 加载数据库url name 为空，则默认数据库
    singleton().load_db_url("", MYSQL_URL);//"mysql://root:TEST@localhost:3306/test"
    //4 加载xml配置

    let f = fs::File::open("./src/example/Example_ActivityMapper.xml");
    singleton().load_xml("Example_ActivityMapper.xml".to_string(), fs::read_to_string("./src/example/Example_ActivityMapper.xml").unwrap());//加载xml数据
}


#[test]
fn test_insert() {
    //初始化rbatis
    let rbatis_opt = init_rbatis();
    if rbatis_opt.is_err() {
        return;
    }
    let mut rbatis = rbatis_opt.unwrap();
    //插入前先删一下
    //let r:Result<i32,String>=rbatis.eval_sql("delete from biz_activity  where id = '1'");

    let activity = Activity {
        id: Some("1".to_string()),
        name: Some("活动1".to_string()),
        pc_link: None,
        h5_link: None,
        pc_banner_img: None,
        h5_banner_img: None,
        sort: Some("12".to_string()),
        status: Some(1),
        remark: None,
        create_time: Some("2019-12-12 00:00:00".to_string()),
        version: Some(1),
        delete_flag: Some(1),
    };
    let r: Result<i32, RbatisError> = rbatis.insert("Example_ActivityMapper.xml", &mut json!(activity));
    println!("[rbatis] result==>  {:?}", r);
}


#[test]
fn test_delete() {
    //初始化rbatis
    let rbatis_opt = init_rbatis();
    if rbatis_opt.is_err() {
        return;
    }
    let mut rbatis = rbatis_opt.unwrap();
    let r: Result<i32, RbatisError> = rbatis.delete("Example_ActivityMapper.xml", &mut json!("1"));
    println!("[rbatis] result==>  {:?}", r);
}

#[test]
fn test_update() {
    //初始化rbatis
    let rbatis_opt = init_rbatis();
    if rbatis_opt.is_err() {
        return;
    }
    let mut rbatis = rbatis_opt.unwrap();
    //先插入
    //插入前先删一下
    let r: i32 = rbatis.raw_sql("delete from biz_activity  where id = '1'").unwrap();
    let r: i32 = rbatis.insert("Example_ActivityMapper.xml", &mut json!(Activity{
        id: Some("1".to_string()),
        name: Some("活动1".to_string()),
        pc_link: None,
        h5_link: None,
        pc_banner_img: None,
        h5_banner_img: None,
        sort: Some("122".to_string()),
        status: Some(1),
        remark: None,
        create_time: Some("2019-12-12 00:00:00".to_string()),
        version: Some(1),
        delete_flag: Some(1)
    })).unwrap();

    //update
    let r: Result<i32, RbatisError> = rbatis.update("Example_ActivityMapper.xml", &mut json!({
    "id":"1",
    "name":"updated",
    }));
    println!("[rbatis] result==>  {:?}", r);
}

#[test]
fn test_update_array() {
    //初始化rbatis
    let rbatis_opt = init_rbatis();
    if rbatis_opt.is_err() {
        return;
    }
    let mut rbatis = rbatis_opt.unwrap();
    //update
    let mut json_arr = json!([Activity{
        id: Some("1".to_string()),
        name: Some("活动1".to_string()),
        pc_link: None,
        h5_link: None,
        pc_banner_img: None,
        h5_banner_img: None,
        sort: Some("1".to_string()),
        status: Some(1),
        remark: None,
        create_time: Some("2019-12-12 00:00:00".to_string()),
        version: Some(1),
        delete_flag: Some(1)
    },Activity{
        id: Some("2".to_string()),
        name: Some("活动2".to_string()),
        pc_link: None,
        h5_link: None,
        pc_banner_img: None,
        h5_banner_img: None,
        sort: Some("1".to_string()),
        status: Some(1),
        remark: None,
        create_time: Some("2019-12-12 00:00:00".to_string()),
        version: Some(1),
        delete_flag: Some(1)
    }]);
    let r: Result<i32, RbatisError> = rbatis.update("Example_ActivityMapper.xml", &mut json_arr);
    println!("[rbatis] result==>  {:?}", r.unwrap());
}

/**
 示例-查询活动 数组 集合
*/
#[test]
fn test_exec_sql() {
    //初始化rbatis
    let rbatis = init_rbatis();
    if rbatis.is_err() {
        return;
    }
    let mut array = vec![];
    //执行到远程mysql 并且获取结果,Result<serde_json::Value, RbatisError>,或者 Result<String, RbatisError> 等任意类型
    let data: Vec<Activity> = rbatis.unwrap().mapper("Example_ActivityMapper.xml", "select_by_condition", &mut json!({
       "name":null,
       "startTime":null,
       "endTime":null,
       "page":null,
       "size":null,
    }), &mut array).unwrap();
    // 写法2，直接运行原生sql
    // let data_opt: Result<serde_json::Value, RbatisError> = rbatis.eval_sql("select * from biz_activity");
    println!("[rbatis] result==>  {:?}", data);
}

/**
分页查询数据
*/
#[test]
fn test_exec_select_page() {
    //初始化rbatis
    let rbatis = init_rbatis();
    if rbatis.is_err() {
        return;
    }
    //执行到远程mysql 并且获取结果,Result<serde_json::Value, RbatisError>,或者 Result<String, RbatisError> 等任意类型
    let data: IPage<Activity> = rbatis.unwrap().select_page("Example_ActivityMapper.xml", &mut json!({
       "name":"新人专享1",
    }), &IPage::new(1, 5)).unwrap();
    println!("[rbatis] result==>  {:?}", data);
}

/**
   自定义分页查询数据(指定xml mapper id)
*/
#[test]
fn test_exec_select_page_custom() {
    //初始化rbatis
    let rbatis = init_rbatis();
    if rbatis.is_err() {
        return;
    }
    //执行到远程mysql 并且获取结果,Result<serde_json::Value, RbatisError>,或者 Result<String, RbatisError> 等任意类型
    let data: IPage<Activity> = rbatis.unwrap().select_page_by_mapper("Example_ActivityMapper.xml", "select_by_page", &mut json!({
       "name":"新人专享",
       "delete_flag": 1,
    }), &IPage::new(1, 5)).unwrap();
    println!("[rbatis] result==>  {:?}", data);
}


/**
   sql中使用py语法(指定xml mapper id)
*/
#[test]
fn test_exec_py_sql() {
    //初始化rbatis
    let rbatis = init_rbatis();
    if rbatis.is_err() {
        return;
    }
    //执行到远程mysql 并且获取结果,Result<serde_json::Value, RbatisError>,或者 Result<String, RbatisError> 等任意类型
    let data: Vec<Activity> = rbatis.unwrap().py_sql("Example_ActivityMapper.xml", &mut json!({
       "name":"新人专享",
       "delete_flag": 1,
    }), "
    SELECT * FROM biz_activity WHERE delete_flag = 1
    if name != null:
      AND name like #{name+'%'}
    ").unwrap();
    println!("[rbatis] result==>  {:?}", data);
}

/**
  测试事务
*/
#[test]
fn test_tx() {
    test_tx_return().unwrap();
}

fn test_tx_return() -> Result<u64, RbatisError> {
    //初始化rbatis
    let rbatis_opt = init_rbatis();
    if rbatis_opt.is_err() {
        return Ok(1);
    }
    let mut rbatis = rbatis_opt.unwrap();
    rbatis.begin("", Propagation::REQUIRED)?;

    let u: u32 = rbatis.raw_sql("UPDATE `biz_activity` SET `name` = '活动1' WHERE (`id` = '2');")?;

    let u: u32 = rbatis.raw_sql("UPDATE `biz_activity` SET `name` = '活动2' WHERE (`id` = '2');")?;

    let u: u32 = rbatis.raw_sql("UPDATE `biz_activity` SET `name` = '活动3' WHERE (`id` = '2');")?;


    let act: Activity = rbatis.raw_sql("select * from biz_activity where id  = '2';")?;
    println!("result:{}", serde_json::to_string(&act).unwrap());


    rbatis.commit("")?;

    return Ok(1);
}


async fn index() -> impl Responder {
    //写法1
    let act: Activity = eval_sql("select * from biz_activity where id  = '2';").unwrap();
    return serde_json::to_string(&act).unwrap();
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    init_singleton_rbatis();
    //初始化rbatis
    HttpServer::new(move || {
        App::new()
            .route("/", web::get().to(index))
    })
        .bind("127.0.0.1:8000")?
        .run()
        .await
}

#[test]
pub fn test_web() {
    //初始化rbatis
    if MYSQL_URL.contains("localhost") {
        return;
    }
    main();
}


pub trait Service {
    fn select_activity(&self) -> Result<Activity, RbatisError>;
    fn update_activity(&mut self) -> Result<String, RbatisError>;
}

struct ServiceImpl {
    select_activity: fn(s: &ServiceImpl) -> Result<Activity, RbatisError>,
    update_activity: fn(s: &mut ServiceImpl) -> Result<String, RbatisError>,
}

impl Service for ServiceImpl {
    impl_service! {
      REQUIRED,  select_activity(&self) -> Result<Activity,RbatisError>
    }
    impl_service_mut! {
      NONE,  update_activity(&mut self) -> Result<String, RbatisError>
    }
}

/// 示例，使用 trait和宏 代理实现服务
#[test]
pub fn test_service() {
    if MYSQL_URL.contains("localhost") {
        return;
    }
    init_singleton_rbatis();

    let mut s = ServiceImpl {
        select_activity: |s: &ServiceImpl| -> Result<Activity, RbatisError>{
            let act: Activity = singleton().raw_sql("select * from biz_activity where id  = '2';").unwrap();
            return Result::Ok(act);
        },
        update_activity: |s: &mut ServiceImpl| -> Result<String, RbatisError>{
            return Result::Ok("ok".to_string());
        },
    };
    let act: Activity = s.select_activity().unwrap();
    println!("{:?}", serde_json::to_string(&act).unwrap().as_str());
    println!("{:?}", s.update_activity().unwrap());
}


//添加 tokio异步支持示例
use tokio::task;
async fn docs(arg: &Value) -> Result<IPage<Activity>, Box<dyn std::error::Error>> {
    let mut new_arg =arg.clone();
    let res = task::spawn_blocking(move || {
        //do some compute-heavy work or call synchronous code
        let data: IPage<Activity> = singleton().select_page("Example_ActivityMapper.xml", &mut new_arg, &IPage::new(1, 5)).unwrap();
        //"done computing"
        return data;
    }).await?;
    Ok(res)
}