use crate::{Captcha, CaptchaProvider};

pub struct PonyCaptcha {}

#[derive(serde::Deserialize, serde::Serialize)]
pub enum Pony {
    TwilightSparkle,
    Rarity,
    Fluttershy,
    RainbowDash,
    PinkiePie,
    Applejack,
}

#[derive(serde::Deserialize, serde::Serialize)]
pub struct PonyCaptchaInstance {}

#[derive(serde::Deserialize)]
pub struct PonyCaptchaSolution([Pony; 6]);

impl CaptchaProvider for PonyCaptcha {
    type Check = PonyCaptchaInstance;
    type Solution = PonyCaptchaSolution;

    fn generate_captcha(&self) -> Captcha<Self::Check> {
        todo!()
    }

    fn verify_captcha(&self, _code: Self::Check, _solution: Self::Solution) -> bool {
        todo!()
    }
}
