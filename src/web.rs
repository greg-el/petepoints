use std::process::Command;
use thirtyfour::prelude::*;

pub async fn start_watcher() -> WebDriverResult<()> {
    // Get webdriver running
    println!("0");

    // match Command::new("chromedriver").arg("--port=4444").output() {
    //     Ok(c) => println!("{:?}", c),
    //     Err(error) => println!("Oh no again, {}", error),
    // }
    println!("1");

    let caps = DesiredCapabilities::chrome();

    let driver = WebDriver::new("http://localhost:4444", caps).await?;
    println!("2");

    driver.get("https://web.whatsapp.com").await?;

    println!("3");
    let bertie = driver.find_element(By::Tag("span")).await?;
    println!("{:?}", bertie);
    println!("hi");

    // driver.quit().await?;
    Ok(())
}
