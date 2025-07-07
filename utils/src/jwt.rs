use serde::{Deserialize, Serialize};
use jsonwebtoken::{DecodingKey, Validation, errors::Error as JwtError, TokenData};

/// JWT (JSON Web Token) claims structure.
///
/// Example:
/// ```json
/// {
///   "iss": "https://auth.myapp.com",
///   "sub": "user_789",
///   "aud": "https://api.myapp.com",
///   "exp": 1700090000,
///
///   "name": "Jane Doe",                    // Public claim
///   "email": "jane@example.com",           // Public claim
///   "user_role": "admin",                  // Private claim
///   "internal_access_level": "superuser"   // Private claim
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct JWTClaims {
    /* Registered claims */

    /// The "iss" (issuer) claim identifies the principal that issued the
    /// JWT.  The processing of this claim is generally application specific.
    /// The "iss" value is a case-sensitive string containing a StringOrURI
    /// value.  Use of this claim is OPTIONAL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iss: Option<String>,

    /// The "sub" (subject) claim identifies the principal that is the
    /// subject of the JWT.  The claims in a JWT are normally statements
    /// about the subject.  The subject value MUST either be scoped to be
    /// locally unique in the context of the issuer or be globally unique.
    /// The processing of this claim is generally application specific.  The
    /// "sub" value is a case-sensitive string containing a StringOrURI
    /// value.  Use of this claim is OPTIONAL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,

    /// The "aud" (audience) claim identifies the recipients that the JWT is
    /// intended for.  Each principal intended to process the JWT MUST
    /// identify itself with a value in the audience claim.  If the principal
    /// processing the claim does not identify itself with a value in the
    /// "aud" claim when this claim is present, then the JWT MUST be
    /// rejected.  In the general case, the "aud" value is an array of case-
    /// sensitive strings, each containing a StringOrURI value.  In the
    /// special case when the JWT has one audience, the "aud" value MAY be a
    /// single case-sensitive string containing a StringOrURI value.  The
    /// interpretation of audience values is generally application specific.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,

    /// The "exp" (expiration time) claim identifies the expiration time on
    /// or after which the JWT MUST NOT be accepted for processing.  The
    /// processing of the "exp" claim requires that the current date/time
    /// MUST be before the expiration date/time listed in the "exp" claim.
    /// Implementers MAY provide for some small leeway, usually no more than
    /// a few minutes, to account for clock skew.  Its value MUST be a number
    /// containing a NumericDate value.  Use of this claim is OPTIONAL.
    #[serde(skip_serializing_if = "is_zero")]
    pub exp: usize,

    /// The "nbf" (not before) claim identifies the time before which the JWT
    /// MUST NOT be accepted for processing.  The processing of the "nbf"
    /// claim requires that the current date/time MUST be after or equal to
    /// the not-before date/time listed in the "nbf" claim.  Implementers MAY
    /// provide for some small leeway, usually no more than a few minutes, to
    /// account for clock skew.  Its value MUST be a number containing a
    /// NumericDate value.  Use of this claim is OPTIONAL.
    #[serde(skip_serializing_if = "is_zero")]
    pub nbf: usize,

    /// The "iat" (issued at) claim identifies the time at which the JWT was
    /// issued.  This claim can be used to determine the age of the JWT.  Its
    /// value MUST be a number containing a NumericDate value.  Use of this
    /// claim is OPTIONAL.
    #[serde(skip_serializing_if = "is_zero")]
    pub iat: usize,

    /// The "jti" (JWT ID) claim provides a unique identifier for the JWT.
    /// The identifier value MUST be assigned in a manner that ensures that
    /// there is a negligible probability that the same value will be
    /// accidentally assigned to a different data object; if the application
    /// uses multiple issuers, collisions MUST be prevented among values
    /// produced by different issuers as well.  The "jti" claim can be used
    /// to prevent the JWT from being replayed.  The "jti" value is a case-
    /// sensitive string.  Use of this claim is OPTIONAL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,

    /* Custom claims */

    /// This claim is required in JWT token `int_fp_jwt` between Interceptor and ForwardProxy.
    /// Used in ForwardProxy to identify the ReverseProxy server.
    #[serde(skip_serializing_if = "Option::is_none", rename(serialize = "upstream", deserialize = "upstream"))]
    pub rp_host: Option<String>,

    /// This claim is required in JWT token `int_rp_jwt` between Interceptor and ReverseProxy.
    #[serde(skip_serializing_if = "Option::is_none", rename(serialize = "sid", deserialize = "sid"))]
    pub ntor_session_id: Option<String>,

    // Additional custom claims can be added here as needed.
}

impl JWTClaims {
    /// Creates a new instance of `JWTClaims` with `iat` set to the current time
    /// and `exp` set to the specified duration in hours.
    pub fn new(duration_hour: Option<i64>) -> Self {
        let exp = match duration_hour {
            Some(hours) => {
                let now = chrono::Utc::now();
                let expiration = now + chrono::Duration::hours(hours);
                expiration.timestamp() as usize
            },
            None => 0, // Default to 0 if no duration is specified
        };

        JWTClaims {
            iss: None,
            sub: None,
            aud: None,
            exp,
            nbf: 0,
            iat: chrono::Utc::now().timestamp() as usize,
            jti: None,
            rp_host: None,
            ntor_session_id: None,
        }
    }

    pub fn set_exp(&mut self, duration_hour: i64) {
        let now = chrono::Utc::now();
        let expiration = now + chrono::Duration::hours(duration_hour);
        self.exp = expiration.timestamp() as usize
    }

    pub fn set_current_iat(&mut self) {
        let now = chrono::Utc::now();
        self.iat = now.timestamp() as usize;
    }
}

fn is_zero(x: &usize) -> bool {
    *x == 0
}

pub fn create_jwt_token(claims: JWTClaims, jwt_secret: &[u8]) -> String {
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(&jwt_secret),
    ).unwrap()
}

pub fn verify_jwt_token(token: &str, jwt_secret: &Vec<u8>) -> Result<TokenData<JWTClaims>, JwtError> {
    jsonwebtoken::decode::<JWTClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_slice()),
        &Validation::default(),
    )
}


