pub mod pony;

pub struct Captcha<Check>
where
    Check: serde::Serialize + serde::de::DeserializeOwned,
{
    /// A Check Code that will be included when verifying the captcha
    ///
    /// This is essentially a way for the captcha provider to reconstruct the original challenge and verify
    /// that the submitted solution is correct
    ///
    /// The check code must be serialized and transmitted to the user
    ///
    /// When submitting the code must be deserialized again.
    ///
    /// The caller is responsible for ensuring that the user does not tamper the Check code.
    pub check: Check,
    /// The HTML code to be included on the page verbatim.
    pub html: maud::PreEscaped<String>,
}

pub trait CaptchaProvider {
    type Check: serde::Serialize + serde::de::DeserializeOwned;

    /// The solution that will be parsed from the captcha parameter of the form submit using URL query parsing.
    /// This automatically will handle arrays for you
    type Solution: serde::de::DeserializeOwned;

    /// Generates a new random captcha with it's challenge code
    fn generate_captcha(&self) -> Captcha<Self::Check>;

    /// Returns true if the captcha is correctly solved, false otherwise
    fn verify_captcha(&self, code: Self::Check, solution: Self::Solution) -> bool;
}
