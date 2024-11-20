use std::thread;
use std::time;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use std::io::Write;
use telegraf::{Client, Point, Metric};

use env_logger::Builder;
use log::{LevelFilter, error, info, debug};
use embedded_hal_bus::i2c;
use embedded_hal_bus::util::AtomicCell;
use ens160_aq::Ens160;
use bme280::i2c::BME280;
use linux_embedded_hal::{Delay, I2cdev};

fn main() {
    let mut builder = Builder::from_default_env();

    builder
        .format(|buf, record| writeln!(buf, "{} - {}", record.level(), record.args()))
        .filter(None, LevelFilter::Debug)
        .init();

    // initialize i2c2 shared bus
    let i2c = I2cdev::new("/dev/i2c-2").unwrap();
    thread::sleep(time::Duration::from_millis(250));
    let i2c_cell = AtomicCell::new(i2c);

    // initialize ens160
    let mut ens160 = Ens160::new_secondary_address(
        i2c::AtomicDevice::new(&i2c_cell), Delay
    );

    // initialize bme280
    let mut bme280 = BME280::new_secondary(
        i2c::AtomicDevice::new(&i2c_cell)
    );
    bme280.init(&mut Delay).unwrap();


    let ens160_result = ens160.initialize();
    thread::sleep(time::Duration::from_millis(250)); //TODO: get wait time from datasheet
    
    match ens160_result {
        Ok(what) => info!("ENS160 initialized ok: {}", what),
        Err(err) => error!("ENS160 initialize error: {:?}", err),
    }

    info!("main loop");
    loop {
        thread::sleep(time::Duration::from_millis(1000));
        if let Ok(status) = ens160.get_status() {
            if status.new_data_ready() {
                let measuremnts_aq = ens160.get_measurements().unwrap();
                info!("co2eq_ppm: {:?}",  measuremnts_aq.co2eq_ppm.get_value()); //CO₂ equivalent (parts per million, ppm)
                info!("tvoc_ppb: {:?}", measuremnts_aq.tvoc_ppb); //Total Volatile Organic Compounds (parts per billion, ppb)
                info!("aiq: {:?}", measuremnts_aq.air_quality_index); // air quality index as enum
                info!("etoh: {:?}", measuremnts_aq.etoh); //ethanol concentration in ppb
            } else {
                debug!("ens160 not ready...")
            }
        }
        let ens160_measurement = bme280.measure(&mut Delay);
        match ens160_measurement {
            Ok(measurements) => {
                info!("Relative Humidity = {}%", measurements.humidity);
                info!("Temperature = {} deg C", measurements.temperature);
                info!("Pressure = {} pascals", measurements.pressure);
            },
            Err(err) => error!("BME280 measurement error {:?}", err),
        }
        write_metric();
    }
}

#[derive(Metric)]
struct MyMetric{
    temp: f32,
    pressure: f32,
    humidity: f32,
    #[telegraf(tag)]
    sensor: String,
}

//#[telegraf(timestamp)]
//ts: u64,

fn write_metric() {

  let mut c = Client::new("unix:///tmp/telegraf.sock").unwrap();
  //let time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
  let point = MyMetric { temp: 10.0 , pressure: 10000000000., humidity: 50.0 , sensor: "ens160".to_string()};

  //let p = Point::new(
  //    String::from("measurement"),a
  //    vec![
  //        (String::from("tag1"), String::from("tag1value"))
  //    ],
  //    vec![
  //        (String::from("field1"), Box::new(10)),
  //        (String::from("field2"), Box::new(20.5)),
  //        (String::from("field3"), Box::new("anything!"))
  //    ],
  //    Some(10)
  //);
  
  c.write(&point).unwrap();
}