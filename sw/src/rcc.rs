use pac::{FLASH, RCC, rcc::vals::Hpre};

pub fn init() {
    // flash needs 3 wait states when core/bus freq >24MHz
    FLASH.actlr().write(|w| w.set_latency(2));
    // set core/bus freq to 1 prescale (48MHz)
    RCC.cfgr0().write(|w| w.set_hpre(Hpre::DIV1));

    time_driver::init();

    // provide USBFS and DMA clocking
    RCC.ahbpcenr().modify(|w| {
        w.set_usbfsen(true);
        w.set_dma1en(true);
    });
    // provide SPI, GPIOA/B/C and AFIO clocking
    RCC.apb2pcenr().modify(|w| {
        w.set_spi1en(true);
        w.set_iopaen(true);
        w.set_iopben(true);
        w.set_iopcen(true);
        w.set_afioen(true);
    });
    // provide UART and TIM2 clocking
    RCC.apb1pcenr().modify(|w| {
        w.set_usart2en(true);
        w.set_tim2en(true);
    });
}
