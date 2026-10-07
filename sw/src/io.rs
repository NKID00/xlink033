use pac::{
    AFIO, GPIOA, GPIOB, GPIOC, TIM2,
    gpio::vals::{Cnf, Mode},
    timer::vals::{CcmrOutputCcs, Ocm},
};

pub fn init() {
    // disable SWD to use those pins as GPIO
    AFIO.pcfr1().modify(|w| w.set_sw_cfg(0b100));

    // enable pull or set output
    GPIOA.bshr().write(|w| {
        for i in 0..=3 {
            w.set_bs(i, true);
        }
        w.set_br(5, true);
        w.set_br(6, true);
        w.set_br(7, true);
        w.set_br(10, true);
        // set PA11 (RST) to low on output to simulate open drain
        w.set_br(11, true);
    });
    GPIOA.cfglr().modify(|w| {
        // PA0-1 LED
        for i in 0..=1 {
            w.set_mode(i, Mode::OUTPUT_50MHZ);
            w.set_cnf(i, Cnf::PULL_IN__AF_PUSH_PULL_OUT);
        }
        // PA2 TX
        w.set_mode(2, Mode::OUTPUT_50MHZ);
        w.set_cnf(2, Cnf::PULL_IN__AF_PUSH_PULL_OUT);
        // PA3 RX
        w.set_mode(3, Mode::INPUT);
        w.set_cnf(3, Cnf::PULL_IN__AF_PUSH_PULL_OUT);
        // PA5 SCK/TCK
        w.set_mode(5, Mode::INPUT);
        w.set_cnf(5, Cnf::FLOATING_IN__OPEN_DRAIN_OUT);
        // PA6 MISO/TDO
        w.set_mode(6, Mode::INPUT);
        w.set_cnf(6, Cnf::FLOATING_IN__OPEN_DRAIN_OUT);
        // PA7 MOSI/TDI
        w.set_mode(7, Mode::INPUT);
        w.set_cnf(7, Cnf::FLOATING_IN__OPEN_DRAIN_OUT);
    });
    GPIOA.cfghr().modify(|w| {
        // PA10 3V3_EN
        w.set_mode(2, Mode::OUTPUT_50MHZ);
        w.set_cnf(2, Cnf::ANALOG_IN__PUSH_PULL_OUT);
        // PA11 RST
        w.set_mode(3, Mode::INPUT);
        w.set_cnf(3, Cnf::FLOATING_IN__OPEN_DRAIN_OUT);
    });

    GPIOB.cfglr().modify(|w| {
        // PB1 MODE
        w.set_mode(1, Mode::INPUT);
        w.set_cnf(1, Cnf::FLOATING_IN__OPEN_DRAIN_OUT);
    });

    // set PC3 (5V_EN) to low
    GPIOC.bshr().write(|w| w.set_bs(3, true));
    // enable pull-up resistor on D+ (PC17)
    GPIOC.bsxr().write(|w| w.set_bs(1, true));
    // disconnect pull-up resistor on D+ (PC17) before device initialization
    AFIO.ctlr().modify(|w| w.set_udp_pue(0b00));
    GPIOC.cfglr().modify(|w| {
        // PC3 5V_EN
        w.set_mode(3, Mode::OUTPUT_50MHZ);
        w.set_cnf(3, Cnf::ANALOG_IN__PUSH_PULL_OUT);
    });
    GPIOC.cfgxr().modify(|w| {
        // PC16 D-
        w.set_mode(0, Mode::INPUT);
        w.set_cnf(0, Cnf::FLOATING_IN__OPEN_DRAIN_OUT);
        // PC17 D+
        w.set_mode(1, Mode::INPUT);
        w.set_cnf(1, Cnf::PULL_IN__AF_PUSH_PULL_OUT);
        // PC18 PIOC_IO0
        w.set_mode(1, Mode::INPUT);
        w.set_cnf(1, Cnf::FLOATING_IN__OPEN_DRAIN_OUT);
        // PC19 PIOC_IO1
        w.set_mode(1, Mode::INPUT);
        w.set_cnf(1, Cnf::FLOATING_IN__OPEN_DRAIN_OUT);
    });
    // enable USB alternative function
    AFIO.ctlr().modify(|w| w.set_usb_ioen(true));

    // enable auto reload
    TIM2.ctlr1().write(|w| {
        w.set_arpe(true);
    });
    // drive timer with hb bus clock
    TIM2.smcfgr().write(|w| {
        w.set_ts(0b000);
        w.set_sms(0b000);
    });
    // prescaler
    TIM2.psc().write_value(47);
    // set auto reload value
    TIM2.atrlr().write_value(99);
    // set channel 1 and 2 to output
    TIM2.chctlr_output(0).write(|w| {
        w.set_ocm(0, Ocm::PWMMODE1);
        // delay channel threshold update until timer overflow
        w.set_ocpe(0, true);
        w.set_ccs(0, CcmrOutputCcs::OUTPUT);
        w.set_ocm(1, Ocm::PWMMODE1);
        w.set_ocpe(1, true);
        w.set_ccs(1, CcmrOutputCcs::OUTPUT);
    });
    TIM2.chcvr(0).write_value(0);
    TIM2.chcvr(1).write_value(0);
    // enable output main switch
    TIM2.bdtr().write(|w| w.set_moe(true));
    // enable channel 1 on PA0 and channel 2 on PA1
    TIM2.ccer().write(|w| {
        w.set_cce(0, true);
        // negative polarity because LED is common anode
        w.set_ccp(0, true);
        w.set_cce(1, true);
        w.set_ccp(1, true);
    });
    // update counter value
    TIM2.swevgr().modify(|w| {
        w.set_ug(true);
    });
    // enable timer
    TIM2.ctlr1().modify(|w| {
        w.set_cen(true);
    });
}

#[inline]
pub fn led_act(duty_cycle: i32) {
    TIM2.chcvr(0).write_value(duty_cycle as u16);
}

#[inline]
pub fn led_act_on() {
    led_act(100);
}

#[inline]
pub fn led_act_off() {
    led_act(0);
}

#[inline]
pub fn led_act_toggle() {
    TIM2.chcvr(0).write(|w| {
        *w = if *w < 50 { 100 } else { 0 };
    });
}

#[inline]
pub fn led_lnk(duty_cycle: i32) {
    TIM2.chcvr(1).write_value(duty_cycle as u16);
}

#[inline]
pub fn led_lnk_on() {
    led_lnk(100);
}

#[inline]
pub fn led_lnk_off() {
    led_lnk(0);
}

#[inline]
pub fn led_lnk_toggle() {
    TIM2.chcvr(1).write(|w| {
        *w = if *w < 50 { 100 } else { 0 };
    });
}
