use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use openssl::{
    aes::{AesKey, unwrap_key, wrap_key},
    bn::{BigNum, BigNumContext},
    ec::{EcGroup, EcKey, EcPoint, PointConversionForm},
    ecdsa::EcdsaSig,
    encrypt::{Decrypter, Encrypter},
    hash::{MessageDigest, hash},
    kdf::{HkdfMode, hkdf},
    md::{Md, MdRef},
    memcmp,
    nid::Nid,
    pkcs5::pbkdf2_hmac,
    pkey::{Id, PKey, Private, Public},
    rand::rand_bytes,
    rsa::{Padding, Rsa},
    sign::{RsaPssSaltlen, Signer, Verifier},
    symm::{Cipher, Crypter, Mode},
};
use rquickjs::{
    Array, Class, Ctx, Exception, JsLifetime, Object, Result as JsResult, TypedArray, Value,
    class::{Trace, Tracer},
};

const SOURCE: &str = include_str!("crypto.js");
const RSA_PUBLIC_EXPONENT: u64 = 65537;

#[derive(Clone, Debug, Default)]
struct Algorithm {
    name: String,
    hash: Option<String>,
    length: Option<usize>,
    modulus_length: Option<usize>,
    public_exponent: Vec<u8>,
    named_curve: Option<String>,
    iv: Vec<u8>,
    additional_data: Vec<u8>,
    tag_length: Option<usize>,
    counter: Vec<u8>,
    salt: Vec<u8>,
    info: Vec<u8>,
    iterations: Option<usize>,
    label: Vec<u8>,
    public: Option<Box<KeyInput>>,
}

#[derive(Clone, Debug)]
struct KeyInput {
    key_type: String,
    extractable: bool,
    algorithm: Algorithm,
    usages: Vec<String>,
    data: Vec<u8>,
}

struct KeyData {
    key_type: &'static str,
    data: Vec<u8>,
}

enum GeneratedKey {
    Single(KeyData),
    Pair { private: KeyData, public: KeyData },
}

/// bridge between the javascript web crypto surface and rust implementations
#[rquickjs::class(rename = "__LampreyCryptoBridge")]
#[derive(Clone, Copy, JsLifetime)]
pub struct CryptoBridge {}

impl<'js> Trace<'js> for CryptoBridge {
    fn trace<'a>(&self, _tracer: Tracer<'a, 'js>) {}
}

/// install the web crypto globals
pub fn setup(ctx: &Ctx<'_>) -> JsResult<()> {
    Class::<CryptoBridge>::define(&ctx.globals())?;
    let bridge = Class::instance(ctx.clone(), CryptoBridge {})?;
    ctx.globals().set("__lampreyCryptoBridge", bridge)?;
    ctx.eval::<(), _>(SOURCE)
}

#[rquickjs::methods]
#[qjs(rename_all = "camelCase")]
impl CryptoBridge {
    #[qjs(constructor)]
    fn new(ctx: Ctx<'_>) -> JsResult<Self> {
        Err(Exception::throw_type(&ctx, "Illegal constructor"))
    }

    fn random_bytes<'js>(&self, ctx: Ctx<'js>, length: usize) -> JsResult<TypedArray<'js, u8>> {
        let mut bytes = vec![0u8; length];
        rand_bytes(&mut bytes).map_err(|error| throw(ctx.clone(), error.to_string()))?;
        TypedArray::new_copy(ctx, bytes)
    }

    fn digest<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        data: Value<'js>,
    ) -> JsResult<TypedArray<'js, u8>> {
        let algorithm = parse_algorithm(&ctx, algorithm)?;
        let data = bytes_from_value(&ctx, data)?;
        let digest = digest_bytes(&ctx, &algorithm, &data)?;
        TypedArray::new_copy(ctx, digest)
    }

    fn sign<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        key: Value<'js>,
        data: Value<'js>,
    ) -> JsResult<TypedArray<'js, u8>> {
        let algorithm = parse_algorithm(&ctx, algorithm)?;
        let key = parse_key(&ctx, key)?;
        let data = bytes_from_value(&ctx, data)?;
        let signature = sign_bytes(&ctx, &algorithm, &key, &data)?;
        TypedArray::new_copy(ctx, signature)
    }

    fn verify<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        key: Value<'js>,
        signature: Value<'js>,
        data: Value<'js>,
    ) -> JsResult<bool> {
        let algorithm = parse_algorithm(&ctx, algorithm)?;
        let key = parse_key(&ctx, key)?;
        let signature = bytes_from_value(&ctx, signature)?;
        let data = bytes_from_value(&ctx, data)?;
        verify_bytes(&ctx, &algorithm, &key, &signature, &data)
    }

    fn encrypt<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        key: Value<'js>,
        data: Value<'js>,
    ) -> JsResult<TypedArray<'js, u8>> {
        let algorithm = parse_algorithm(&ctx, algorithm)?;
        let key = parse_key(&ctx, key)?;
        let data = bytes_from_value(&ctx, data)?;
        let encrypted = encrypt_bytes(&ctx, &algorithm, &key, &data)?;
        TypedArray::new_copy(ctx, encrypted)
    }

    fn decrypt<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        key: Value<'js>,
        data: Value<'js>,
    ) -> JsResult<TypedArray<'js, u8>> {
        let algorithm = parse_algorithm(&ctx, algorithm)?;
        let key = parse_key(&ctx, key)?;
        let data = bytes_from_value(&ctx, data)?;
        let decrypted = decrypt_bytes(&ctx, &algorithm, &key, &data)?;
        TypedArray::new_copy(ctx, decrypted)
    }

    fn generate_key<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        extractable: bool,
        usages: Value<'js>,
    ) -> JsResult<Value<'js>> {
        let algorithm = parse_algorithm(&ctx, algorithm)?;
        let usages = parse_usages(&ctx, usages)?;
        let generated = generate_key(&ctx, &algorithm)?;

        match generated {
            GeneratedKey::Single(key) => {
                key_to_value(&ctx, key.key_type, extractable, &usages, &key.data)
            }
            GeneratedKey::Pair { private, public } => {
                let object = Object::new(ctx.clone())?;
                object.set(
                    "privateKey",
                    key_to_value(&ctx, private.key_type, extractable, &usages, &private.data)?,
                )?;
                object.set(
                    "publicKey",
                    key_to_value(&ctx, public.key_type, true, &[], &public.data)?,
                )?;
                Ok(object.into_value())
            }
        }
    }

    fn import_key<'js>(
        &self,
        ctx: Ctx<'js>,
        format: String,
        key_data: Value<'js>,
        algorithm: Value<'js>,
        extractable: bool,
        usages: Value<'js>,
    ) -> JsResult<Value<'js>> {
        let algorithm = parse_algorithm(&ctx, algorithm)?;
        let usages = parse_usages(&ctx, usages)?;
        let (key_type, data) = import_key(&ctx, &format, key_data, &algorithm)?;
        key_to_value(&ctx, key_type, extractable, &usages, &data)
    }

    fn export_key<'js>(
        &self,
        ctx: Ctx<'js>,
        format: String,
        key: Value<'js>,
    ) -> JsResult<Value<'js>> {
        let key = parse_key(&ctx, key)?;
        export_key(&ctx, &format, &key)
    }

    fn derive_bits<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        base_key: Value<'js>,
        length: Option<usize>,
    ) -> JsResult<TypedArray<'js, u8>> {
        let algorithm = parse_algorithm(&ctx, algorithm)?;
        let base_key = parse_key(&ctx, base_key)?;
        let length = length.unwrap_or(0);
        let bits = derive_bits(&ctx, &algorithm, &base_key, length)?;
        TypedArray::new_copy(ctx, bits)
    }

    fn wrap_key<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        wrapping_key: Value<'js>,
        data: Value<'js>,
    ) -> JsResult<TypedArray<'js, u8>> {
        let algorithm = parse_algorithm(&ctx, algorithm)?;
        let wrapping_key = parse_key(&ctx, wrapping_key)?;
        let data = bytes_from_value(&ctx, data)?;
        let wrapped = wrap_key_bytes(&ctx, &algorithm, &wrapping_key, &data)?;
        TypedArray::new_copy(ctx, wrapped)
    }

    fn unwrap_key<'js>(
        &self,
        ctx: Ctx<'js>,
        algorithm: Value<'js>,
        unwrapping_key: Value<'js>,
        data: Value<'js>,
    ) -> JsResult<TypedArray<'js, u8>> {
        let algorithm = parse_algorithm(&ctx, algorithm)?;
        let unwrapping_key = parse_key(&ctx, unwrapping_key)?;
        let data = bytes_from_value(&ctx, data)?;
        let unwrapped = unwrap_key_bytes(&ctx, &algorithm, &unwrapping_key, &data)?;
        TypedArray::new_copy(ctx, unwrapped)
    }
}

fn parse_algorithm<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Algorithm> {
    if let Some(name) = value.as_string() {
        return Ok(Algorithm {
            name: name.to_string()?,
            ..Default::default()
        });
    }

    let Some(object) = value.into_object() else {
        return Err(Exception::throw_type(
            ctx,
            "algorithm must be a string or object",
        ));
    };

    let mut algorithm = Algorithm {
        name: object.get::<_, String>("name")?,
        ..Default::default()
    };

    if let Some(hash) = object.get::<_, Option<Value>>("hash")? {
        algorithm.hash = parse_hash(ctx, hash)?;
    }

    algorithm.length = object.get::<_, Option<usize>>("length")?;
    algorithm.modulus_length = object.get::<_, Option<usize>>("modulusLength")?;
    algorithm.named_curve = object.get::<_, Option<String>>("namedCurve")?;
    algorithm.tag_length = object.get::<_, Option<usize>>("tagLength")?;
    algorithm.iterations = object.get::<_, Option<usize>>("iterations")?;

    for (field, target) in [
        ("iv", &mut algorithm.iv),
        ("additionalData", &mut algorithm.additional_data),
        ("counter", &mut algorithm.counter),
        ("salt", &mut algorithm.salt),
        ("info", &mut algorithm.info),
        ("label", &mut algorithm.label),
        ("publicExponent", &mut algorithm.public_exponent),
    ] {
        if let Some(value) = object.get::<_, Option<Value>>(field)? {
            *target = bytes_from_value(ctx, value)?;
        }
    }

    if let Some(public) = object.get::<_, Option<Value>>("public")? {
        algorithm.public = Some(Box::new(parse_key(ctx, public)?));
    }

    Ok(algorithm)
}

fn parse_hash<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Option<String>> {
    if let Some(name) = value.as_string() {
        return Ok(Some(name.to_string()?));
    }

    if let Some(object) = value.as_object() {
        return Ok(Some(object.get::<_, String>("name")?));
    }

    Err(Exception::throw_type(
        ctx,
        "hash must be a string or object",
    ))
}

fn parse_usages<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Vec<String>> {
    let array = value
        .into_array()
        .ok_or_else(|| Exception::throw_type(ctx, "key usages must be an array"))?;
    array.iter::<String>().collect()
}

fn parse_key<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<KeyInput> {
    let object = value
        .into_object()
        .ok_or_else(|| Exception::throw_type(ctx, "expected a CryptoKey"))?;

    Ok(KeyInput {
        key_type: object.get::<_, String>("type")?,
        extractable: object.get::<_, bool>("extractable")?,
        algorithm: parse_algorithm(ctx, object.get::<_, Value>("algorithm")?)?,
        usages: parse_usages(ctx, object.get::<_, Value>("usages")?)?,
        data: bytes_from_value(ctx, object.get::<_, Value>("data")?)?,
    })
}

fn key_to_value<'js>(
    ctx: &Ctx<'js>,
    key_type: &str,
    extractable: bool,
    usages: &[String],
    data: &[u8],
) -> JsResult<Value<'js>> {
    let object = Object::new(ctx.clone())?;
    object.set("type", key_type)?;
    object.set("extractable", extractable)?;

    let usages_array = Array::new(ctx.clone())?;
    for (index, usage) in usages.iter().enumerate() {
        usages_array.set(index, usage.as_str())?;
    }
    object.set("usages", usages_array)?;
    object.set("data", TypedArray::new_copy(ctx.clone(), data)?)?;

    Ok(object.into_value())
}

fn import_key<'js>(
    ctx: &Ctx<'js>,
    format: &str,
    key_data: Value<'js>,
    algorithm: &Algorithm,
) -> JsResult<(&'static str, Vec<u8>)> {
    let name = algorithm.name.to_ascii_uppercase();

    match format {
        "raw" => {
            let bytes = bytes_from_value(ctx, key_data)?;

            if is_secret_algorithm(&name) {
                return Ok(("secret", bytes));
            }

            if name == "ED25519" || name == "X25519" {
                let id = if name == "ED25519" {
                    Id::ED25519
                } else {
                    Id::X25519
                };
                let key = PKey::public_key_from_raw_bytes(&bytes, id)
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                let der = key
                    .public_key_to_der()
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                return Ok(("public", der));
            }

            if name == "ECDSA" || name == "ECDH" {
                let curve = named_curve(ctx, algorithm)?;
                let group = EcGroup::from_curve_name(curve)
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                let mut context =
                    BigNumContext::new().map_err(|error| throw(ctx.clone(), error.to_string()))?;
                let point = EcPoint::from_bytes(&group, &bytes, &mut context)
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                let key = EcKey::from_public_key(&group, &point)
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                let key = PKey::from_ec_key(key)
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                let der = key
                    .public_key_to_der()
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                return Ok(("public", der));
            }

            Err(not_supported(ctx, "raw key import"))
        }
        "spki" => {
            let bytes = bytes_from_value(ctx, key_data)?;
            PKey::public_key_from_der(&bytes)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            Ok(("public", bytes))
        }
        "pkcs8" => {
            let bytes = bytes_from_value(ctx, key_data)?;
            PKey::private_key_from_pkcs8(&bytes)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            Ok(("private", bytes))
        }
        "jwk" => import_jwk(ctx, key_data, algorithm),
        _ => Err(not_supported(ctx, "key format")),
    }
}

fn export_key<'js>(ctx: &Ctx<'js>, format: &str, key: &KeyInput) -> JsResult<Value<'js>> {
    match format {
        "raw" => export_raw(ctx, key),
        "spki" => {
            let public = public_pkey(ctx, key)?;
            let der = public
                .public_key_to_der()
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            Ok(TypedArray::new_copy(ctx.clone(), der)?.into_value())
        }
        "pkcs8" => {
            let private = private_pkey(ctx, key)?;
            let der = private
                .private_key_to_pkcs8()
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            Ok(TypedArray::new_copy(ctx.clone(), der)?.into_value())
        }
        "jwk" => export_jwk(ctx, key),
        _ => Err(not_supported(ctx, "key format")),
    }
}

fn export_raw<'js>(ctx: &Ctx<'js>, key: &KeyInput) -> JsResult<Value<'js>> {
    let name = key.algorithm.name.to_ascii_uppercase();

    if key.key_type == "secret" {
        return Ok(TypedArray::new_copy(ctx.clone(), key.data.as_slice())?.into_value());
    }

    if key.key_type == "public" && (name == "ED25519" || name == "X25519") {
        let key = public_pkey(ctx, key)?;
        let raw = key
            .raw_public_key()
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        return Ok(TypedArray::new_copy(ctx.clone(), raw)?.into_value());
    }

    if key.key_type == "public" && (name == "ECDSA" || name == "ECDH") {
        let key = public_pkey(ctx, key)?;
        let ec = key
            .ec_key()
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let mut context =
            BigNumContext::new().map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let bytes = ec
            .public_key()
            .to_bytes(ec.group(), PointConversionForm::UNCOMPRESSED, &mut context)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        return Ok(TypedArray::new_copy(ctx.clone(), bytes)?.into_value());
    }

    Err(not_supported(ctx, "raw key export"))
}

fn generate_key(ctx: &Ctx<'_>, algorithm: &Algorithm) -> JsResult<GeneratedKey> {
    let name = algorithm.name.to_ascii_uppercase();

    if name.starts_with("AES-") {
        let bits = algorithm.length.unwrap_or(256);
        let mut data = vec![0u8; bits / 8];
        rand_bytes(&mut data).map_err(|error| throw(ctx.clone(), error.to_string()))?;
        return Ok(GeneratedKey::Single(KeyData {
            key_type: "secret",
            data,
        }));
    }

    if name == "HMAC" {
        let hash = algorithm
            .hash
            .as_deref()
            .ok_or_else(|| Exception::throw_type(ctx, "HMAC requires a hash"))?;
        let bits = algorithm.length.unwrap_or_else(|| hash_block_len(hash) * 8);
        let mut data = vec![0u8; bits / 8];
        rand_bytes(&mut data).map_err(|error| throw(ctx.clone(), error.to_string()))?;
        return Ok(GeneratedKey::Single(KeyData {
            key_type: "secret",
            data,
        }));
    }

    if name == "RSA-OAEP" || name == "RSA-PSS" || name == "RSASSA-PKCS1-V1_5" {
        let bits = algorithm
            .modulus_length
            .or(algorithm.length)
            .unwrap_or(2048) as u32;
        let exponent = if algorithm.public_exponent.is_empty() {
            BigNum::from_u32(RSA_PUBLIC_EXPONENT as u32)
        } else {
            BigNum::from_slice(&algorithm.public_exponent)
        }
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let rsa = Rsa::generate_with_e(bits, &exponent)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let private = PKey::from_rsa(rsa).map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let public = private
            .public_key_to_der()
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let private = private
            .private_key_to_pkcs8()
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        return Ok(GeneratedKey::Pair {
            private: KeyData {
                key_type: "private",
                data: private,
            },
            public: KeyData {
                key_type: "public",
                data: public,
            },
        });
    }

    if name == "ECDSA" || name == "ECDH" {
        let group = EcGroup::from_curve_name(named_curve(ctx, algorithm)?)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let key = EcKey::generate(&group).map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let private =
            PKey::from_ec_key(key).map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let public = private
            .public_key_to_der()
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let private = private
            .private_key_to_pkcs8()
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        return Ok(GeneratedKey::Pair {
            private: KeyData {
                key_type: "private",
                data: private,
            },
            public: KeyData {
                key_type: "public",
                data: public,
            },
        });
    }

    if name == "ED25519" || name == "X25519" {
        let key = if name == "ED25519" {
            PKey::generate_ed25519()
        } else {
            PKey::generate_x25519()
        }
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let public = key
            .public_key_to_der()
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let private = key
            .private_key_to_pkcs8()
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        return Ok(GeneratedKey::Pair {
            private: KeyData {
                key_type: "private",
                data: private,
            },
            public: KeyData {
                key_type: "public",
                data: public,
            },
        });
    }

    Err(not_supported(ctx, "generateKey"))
}

fn digest_bytes(ctx: &Ctx<'_>, algorithm: &Algorithm, data: &[u8]) -> JsResult<Vec<u8>> {
    let name = algorithm.hash.as_deref().unwrap_or(&algorithm.name);
    let md = message_digest(name).ok_or_else(|| not_supported(ctx, "digest algorithm"))?;
    hash(md, data)
        .map(|digest| digest.to_vec())
        .map_err(|error| throw(ctx.clone(), error.to_string()))
}

fn sign_bytes(
    ctx: &Ctx<'_>,
    algorithm: &Algorithm,
    key: &KeyInput,
    data: &[u8],
) -> JsResult<Vec<u8>> {
    let name = algorithm.name.to_ascii_uppercase();

    if name == "HMAC" {
        let hash = algorithm
            .hash
            .as_deref()
            .or(key.algorithm.hash.as_deref())
            .ok_or_else(|| Exception::throw_type(ctx, "HMAC requires a hash"))?;
        let md = message_digest(hash).ok_or_else(|| not_supported(ctx, "HMAC hash"))?;
        let pkey = PKey::hmac(&key.data).map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let mut signer =
            Signer::new(md, &pkey).map_err(|error| throw(ctx.clone(), error.to_string()))?;
        signer
            .update(data)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        return signer
            .sign_to_vec()
            .map_err(|error| throw(ctx.clone(), error.to_string()));
    }

    let private = private_pkey(ctx, key)?;

    if name == "RSASSA-PKCS1-V1_5" || name == "RSA-PSS" {
        let hash = key_hash(ctx, key, algorithm)?;
        let md = message_digest(hash).ok_or_else(|| not_supported(ctx, "RSA hash"))?;
        let mut signer =
            Signer::new(md, &private).map_err(|error| throw(ctx.clone(), error.to_string()))?;
        if name == "RSA-PSS" {
            signer
                .set_rsa_padding(Padding::PKCS1_PSS)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            signer
                .set_rsa_pss_saltlen(RsaPssSaltlen::DIGEST_LENGTH)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            signer
                .set_rsa_mgf1_md(md)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        } else {
            signer
                .set_rsa_padding(Padding::PKCS1)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        }
        signer
            .update(data)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        return signer
            .sign_to_vec()
            .map_err(|error| throw(ctx.clone(), error.to_string()));
    }

    if name == "ECDSA" {
        let hash = algorithm
            .hash
            .as_deref()
            .ok_or_else(|| Exception::throw_type(ctx, "ECDSA requires a hash"))?;
        let md = message_digest(hash).ok_or_else(|| not_supported(ctx, "ECDSA hash"))?;
        let mut signer =
            Signer::new(md, &private).map_err(|error| throw(ctx.clone(), error.to_string()))?;
        signer
            .update(data)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let der = signer
            .sign_to_vec()
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let signature =
            EcdsaSig::from_der(&der).map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let size = coordinate_size(ctx, &key.algorithm)?;
        return Ok(ecdsa_signature_to_raw(
            ctx,
            signature.r(),
            signature.s(),
            size,
        )?);
    }

    if name == "ED25519" {
        let mut signer = Signer::new_without_digest(&private)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        return signer
            .sign_oneshot_to_vec(data)
            .map_err(|error| throw(ctx.clone(), error.to_string()));
    }

    Err(not_supported(ctx, "sign algorithm"))
}

fn verify_bytes(
    ctx: &Ctx<'_>,
    algorithm: &Algorithm,
    key: &KeyInput,
    signature: &[u8],
    data: &[u8],
) -> JsResult<bool> {
    let name = algorithm.name.to_ascii_uppercase();

    if name == "HMAC" {
        let hash = algorithm
            .hash
            .as_deref()
            .or(key.algorithm.hash.as_deref())
            .ok_or_else(|| Exception::throw_type(ctx, "HMAC requires a hash"))?;
        let md = message_digest(hash).ok_or_else(|| not_supported(ctx, "HMAC hash"))?;
        let pkey = PKey::hmac(&key.data).map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let mut signer =
            Signer::new(md, &pkey).map_err(|error| throw(ctx.clone(), error.to_string()))?;
        signer
            .update(data)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let expected = signer
            .sign_to_vec()
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        return Ok(memcmp::eq(&expected, signature));
    }

    let public = public_pkey(ctx, key)?;

    if name == "RSASSA-PKCS1-V1_5" || name == "RSA-PSS" {
        let hash = key_hash(ctx, key, algorithm)?;
        let md = message_digest(hash).ok_or_else(|| not_supported(ctx, "RSA hash"))?;
        let mut verifier =
            Verifier::new(md, &public).map_err(|error| throw(ctx.clone(), error.to_string()))?;
        if name == "RSA-PSS" {
            verifier
                .set_rsa_padding(Padding::PKCS1_PSS)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            verifier
                .set_rsa_pss_saltlen(RsaPssSaltlen::DIGEST_LENGTH)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            verifier
                .set_rsa_mgf1_md(md)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        } else {
            verifier
                .set_rsa_padding(Padding::PKCS1)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        }
        verifier
            .update(data)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        return verifier
            .verify(signature)
            .map_err(|error| throw(ctx.clone(), error.to_string()));
    }

    if name == "ECDSA" {
        let hash = algorithm
            .hash
            .as_deref()
            .ok_or_else(|| Exception::throw_type(ctx, "ECDSA requires a hash"))?;
        let md = message_digest(hash).ok_or_else(|| not_supported(ctx, "ECDSA hash"))?;
        let size = coordinate_size(ctx, &key.algorithm)?;
        let der = ecdsa_signature_from_raw(ctx, signature, size)?;
        let mut verifier =
            Verifier::new(md, &public).map_err(|error| throw(ctx.clone(), error.to_string()))?;
        verifier
            .update(data)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        return verifier
            .verify(&der)
            .map_err(|error| throw(ctx.clone(), error.to_string()));
    }

    if name == "ED25519" {
        let mut verifier = Verifier::new_without_digest(&public)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        return verifier
            .verify_oneshot(signature, data)
            .map_err(|error| throw(ctx.clone(), error.to_string()));
    }

    Err(not_supported(ctx, "verify algorithm"))
}

fn encrypt_bytes(
    ctx: &Ctx<'_>,
    algorithm: &Algorithm,
    key: &KeyInput,
    data: &[u8],
) -> JsResult<Vec<u8>> {
    let name = algorithm.name.to_ascii_uppercase();

    if name == "RSA-OAEP" {
        let public = public_pkey(ctx, key)?;
        let hash = key_hash(ctx, key, algorithm)?;
        let md = message_digest(hash).ok_or_else(|| not_supported(ctx, "RSA-OAEP hash"))?;
        let mut encrypter =
            Encrypter::new(&public).map_err(|error| throw(ctx.clone(), error.to_string()))?;
        encrypter
            .set_rsa_padding(Padding::PKCS1_OAEP)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        encrypter
            .set_rsa_oaep_md(md)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        encrypter
            .set_rsa_mgf1_md(md)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        if !algorithm.label.is_empty() {
            encrypter
                .set_rsa_oaep_label(&algorithm.label)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        }
        let mut output = vec![
            0u8;
            encrypter
                .encrypt_len(data)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?
        ];
        let len = encrypter
            .encrypt(data, &mut output)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        output.truncate(len);
        return Ok(output);
    }

    if name == "AES-GCM" {
        return aes_gcm_encrypt(ctx, algorithm, &key.data, data);
    }
    if name == "AES-CBC" {
        return aes_mode_encrypt(
            ctx,
            aes_mode_cipher(ctx, false, &key.data)?,
            algorithm,
            &key.data,
            data,
        );
    }
    if name == "AES-CTR" {
        return aes_mode_encrypt(
            ctx,
            aes_mode_cipher(ctx, true, &key.data)?,
            algorithm,
            &key.data,
            data,
        );
    }

    Err(not_supported(ctx, "encrypt algorithm"))
}

fn decrypt_bytes(
    ctx: &Ctx<'_>,
    algorithm: &Algorithm,
    key: &KeyInput,
    data: &[u8],
) -> JsResult<Vec<u8>> {
    let name = algorithm.name.to_ascii_uppercase();

    if name == "RSA-OAEP" {
        let private = private_pkey(ctx, key)?;
        let hash = key_hash(ctx, key, algorithm)?;
        let md = message_digest(hash).ok_or_else(|| not_supported(ctx, "RSA-OAEP hash"))?;
        let mut decrypter =
            Decrypter::new(&private).map_err(|error| throw(ctx.clone(), error.to_string()))?;
        decrypter
            .set_rsa_padding(Padding::PKCS1_OAEP)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        decrypter
            .set_rsa_oaep_md(md)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        decrypter
            .set_rsa_mgf1_md(md)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        if !algorithm.label.is_empty() {
            decrypter
                .set_rsa_oaep_label(&algorithm.label)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        }
        let mut output = vec![
            0u8;
            decrypter
                .decrypt_len(data)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?
        ];
        let len = decrypter
            .decrypt(data, &mut output)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        output.truncate(len);
        return Ok(output);
    }

    if name == "AES-GCM" {
        return aes_gcm_decrypt(ctx, algorithm, &key.data, data);
    }
    if name == "AES-CBC" {
        return aes_mode_decrypt(
            ctx,
            aes_mode_cipher(ctx, false, &key.data)?,
            algorithm,
            &key.data,
            data,
        );
    }
    if name == "AES-CTR" {
        return aes_mode_decrypt(
            ctx,
            aes_mode_cipher(ctx, true, &key.data)?,
            algorithm,
            &key.data,
            data,
        );
    }

    Err(not_supported(ctx, "decrypt algorithm"))
}

fn derive_bits(
    ctx: &Ctx<'_>,
    algorithm: &Algorithm,
    base_key: &KeyInput,
    length: usize,
) -> JsResult<Vec<u8>> {
    let name = algorithm.name.to_ascii_uppercase();
    let output_len = length / 8;

    if name == "HKDF" {
        let hash = algorithm
            .hash
            .as_deref()
            .ok_or_else(|| Exception::throw_type(ctx, "HKDF requires a hash"))?;
        let md = md(hash).ok_or_else(|| not_supported(ctx, "HKDF hash"))?;
        let mut output = vec![0u8; output_len];
        hkdf(
            md,
            &base_key.data,
            Some(&algorithm.salt),
            Some(&algorithm.info),
            HkdfMode::ExtractAndExpand,
            None,
            &mut output,
        )
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        return Ok(output);
    }

    if name == "PBKDF2" {
        let hash = algorithm
            .hash
            .as_deref()
            .ok_or_else(|| Exception::throw_type(ctx, "PBKDF2 requires a hash"))?;
        let md = message_digest(hash).ok_or_else(|| not_supported(ctx, "PBKDF2 hash"))?;
        let iterations = algorithm
            .iterations
            .filter(|iterations| *iterations > 0)
            .ok_or_else(|| Exception::throw_type(ctx, "PBKDF2 requires iterations"))?;
        let mut output = vec![0u8; output_len];
        pbkdf2_hmac(&base_key.data, &algorithm.salt, iterations, md, &mut output)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        return Ok(output);
    }

    if name == "ECDH" || name == "X25519" {
        let private = private_pkey(ctx, base_key)?;
        let peer = algorithm
            .public
            .as_ref()
            .ok_or_else(|| Exception::throw_type(ctx, "derive algorithm requires a public key"))?;
        let peer = public_pkey(ctx, peer)?;
        let mut deriver = openssl::derive::Deriver::new(&private)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        deriver
            .set_peer(&peer)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        let output = deriver
            .derive_to_vec()
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
        if output.len() < output_len {
            return Err(throw(
                ctx.clone(),
                "requested length exceeds the derived key length".to_string(),
            ));
        }
        return Ok(output[..output_len].to_vec());
    }

    Err(not_supported(ctx, "deriveBits algorithm"))
}

fn wrap_key_bytes(
    ctx: &Ctx<'_>,
    algorithm: &Algorithm,
    wrapping_key: &KeyInput,
    data: &[u8],
) -> JsResult<Vec<u8>> {
    if !algorithm.name.eq_ignore_ascii_case("AES-KW") {
        return encrypt_bytes(ctx, algorithm, wrapping_key, data);
    }

    let key = AesKey::new_encrypt(&wrapping_key.data)
        .map_err(|_| throw(ctx.clone(), "invalid AES-KW key".to_string()))?;
    let mut output = vec![0u8; data.len() + 8];
    let written = wrap_key(&key, None, &mut output, data)
        .map_err(|_| throw(ctx.clone(), "AES-KW wrap failed".to_string()))?;
    output.truncate(written);
    Ok(output)
}

fn unwrap_key_bytes(
    ctx: &Ctx<'_>,
    algorithm: &Algorithm,
    unwrapping_key: &KeyInput,
    data: &[u8],
) -> JsResult<Vec<u8>> {
    if !algorithm.name.eq_ignore_ascii_case("AES-KW") {
        return decrypt_bytes(ctx, algorithm, unwrapping_key, data);
    }
    if data.len() < 16 {
        return Err(Exception::throw_type(ctx, "invalid wrapped key"));
    }

    let key = AesKey::new_decrypt(&unwrapping_key.data)
        .map_err(|_| throw(ctx.clone(), "invalid AES-KW key".to_string()))?;
    let mut output = vec![0u8; data.len() - 8];
    let written = unwrap_key(&key, None, &mut output, data)
        .map_err(|_| throw(ctx.clone(), "AES-KW unwrap failed".to_string()))?;
    output.truncate(written);
    Ok(output)
}

fn aes_gcm_encrypt(
    ctx: &Ctx<'_>,
    algorithm: &Algorithm,
    key: &[u8],
    data: &[u8],
) -> JsResult<Vec<u8>> {
    let cipher =
        aes_gcm_cipher(key).ok_or_else(|| throw(ctx.clone(), "invalid AES key".to_string()))?;
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, Some(&algorithm.iv))
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    crypter.pad(false);
    if !algorithm.additional_data.is_empty() {
        crypter
            .aad_update(&algorithm.additional_data)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    }

    let mut output = vec![0u8; data.len() + cipher.block_size()];
    let mut written = crypter
        .update(data, &mut output)
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    written += crypter
        .finalize(&mut output[written..])
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    output.truncate(written);

    let tag_len = gcm_tag_len(ctx, algorithm)? / 8;
    let mut tag = vec![0u8; tag_len];
    crypter
        .get_tag(&mut tag)
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    output.extend_from_slice(&tag[..tag_len]);
    Ok(output)
}

fn aes_gcm_decrypt(
    ctx: &Ctx<'_>,
    algorithm: &Algorithm,
    key: &[u8],
    data: &[u8],
) -> JsResult<Vec<u8>> {
    let tag_len = gcm_tag_len(ctx, algorithm)? / 8;
    if data.len() < tag_len {
        return Err(Exception::throw_type(ctx, "ciphertext is too short"));
    }
    let (data, tag) = data.split_at(data.len() - tag_len);
    let cipher =
        aes_gcm_cipher(key).ok_or_else(|| throw(ctx.clone(), "invalid AES key".to_string()))?;
    let mut crypter = Crypter::new(cipher, Mode::Decrypt, key, Some(&algorithm.iv))
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    crypter.pad(false);
    crypter
        .set_tag(tag)
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    if !algorithm.additional_data.is_empty() {
        crypter
            .aad_update(&algorithm.additional_data)
            .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    }

    let mut output = vec![0u8; data.len() + cipher.block_size()];
    let mut written = crypter
        .update(data, &mut output)
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    written += crypter
        .finalize(&mut output[written..])
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    output.truncate(written);
    Ok(output)
}

fn aes_mode_encrypt(
    ctx: &Ctx<'_>,
    cipher: Cipher,
    algorithm: &Algorithm,
    key: &[u8],
    data: &[u8],
) -> JsResult<Vec<u8>> {
    let mut crypter = Crypter::new(cipher, Mode::Encrypt, key, Some(&algorithm.iv))
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    crypter.pad(cipher.block_size() != 1);
    let mut output = vec![0u8; data.len() + cipher.block_size()];
    let mut written = crypter
        .update(data, &mut output)
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    written += crypter
        .finalize(&mut output[written..])
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    output.truncate(written);
    Ok(output)
}

fn aes_mode_decrypt(
    ctx: &Ctx<'_>,
    cipher: Cipher,
    algorithm: &Algorithm,
    key: &[u8],
    data: &[u8],
) -> JsResult<Vec<u8>> {
    let mut crypter = Crypter::new(cipher, Mode::Decrypt, key, Some(&algorithm.iv))
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    crypter.pad(cipher.block_size() != 1);
    let mut output = vec![0u8; data.len() + cipher.block_size()];
    let mut written = crypter
        .update(data, &mut output)
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    written += crypter
        .finalize(&mut output[written..])
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    output.truncate(written);
    Ok(output)
}

fn gcm_tag_len(ctx: &Ctx<'_>, algorithm: &Algorithm) -> JsResult<usize> {
    match algorithm.tag_length.unwrap_or(128) {
        128 | 120 | 112 | 104 | 96 => Ok(algorithm.tag_length.unwrap_or(128)),
        _ => Err(Exception::throw_type(ctx, "invalid AES-GCM tag length")),
    }
}

fn aes_gcm_cipher(key: &[u8]) -> Option<Cipher> {
    match key.len() {
        16 => Some(Cipher::aes_128_gcm()),
        24 => Some(Cipher::aes_192_gcm()),
        32 => Some(Cipher::aes_256_gcm()),
        _ => None,
    }
}

fn aes_mode_cipher(ctx: &Ctx<'_>, ctr: bool, key: &[u8]) -> JsResult<Cipher> {
    match (ctr, key.len()) {
        (false, 16) => Ok(Cipher::aes_128_cbc()),
        (false, 24) => Ok(Cipher::aes_192_cbc()),
        (false, 32) => Ok(Cipher::aes_256_cbc()),
        (true, 16) => Ok(Cipher::aes_128_ctr()),
        (true, 24) => Ok(Cipher::aes_192_ctr()),
        (true, 32) => Ok(Cipher::aes_256_ctr()),
        _ => Err(throw(ctx.clone(), "invalid AES key".to_string())),
    }
}

fn private_pkey(ctx: &Ctx<'_>, key: &KeyInput) -> JsResult<PKey<Private>> {
    if key.key_type != "private" {
        return Err(Exception::throw_type(ctx, "expected a private key"));
    }
    PKey::private_key_from_pkcs8(&key.data).map_err(|error| throw(ctx.clone(), error.to_string()))
}

fn public_pkey(ctx: &Ctx<'_>, key: &KeyInput) -> JsResult<PKey<Public>> {
    if key.key_type == "public" {
        return PKey::public_key_from_der(&key.data)
            .map_err(|error| throw(ctx.clone(), error.to_string()));
    }

    let private = private_pkey(ctx, key)?;
    PKey::public_key_from_der(
        &private
            .public_key_to_der()
            .map_err(|error| throw(ctx.clone(), error.to_string()))?,
    )
    .map_err(|error| throw(ctx.clone(), error.to_string()))
}

fn key_hash<'a>(ctx: &Ctx<'_>, key: &'a KeyInput, algorithm: &'a Algorithm) -> JsResult<&'a str> {
    key.algorithm
        .hash
        .as_deref()
        .or(algorithm.hash.as_deref())
        .ok_or_else(|| throw(ctx.clone(), "missing hash".to_string()))
}

fn message_digest(name: &str) -> Option<MessageDigest> {
    match name.to_ascii_uppercase().replace('_', "-").as_str() {
        "SHA-1" => Some(MessageDigest::sha1()),
        "SHA-256" => Some(MessageDigest::sha256()),
        "SHA-384" => Some(MessageDigest::sha384()),
        "SHA-512" => Some(MessageDigest::sha512()),
        _ => None,
    }
}

fn md(name: &str) -> Option<&'static MdRef> {
    match name.to_ascii_uppercase().replace('_', "-").as_str() {
        "SHA-1" => Some(Md::sha1()),
        "SHA-256" => Some(Md::sha256()),
        "SHA-384" => Some(Md::sha384()),
        "SHA-512" => Some(Md::sha512()),
        _ => None,
    }
}

fn hash_block_len(hash: &str) -> usize {
    match hash.to_ascii_uppercase().replace('_', "-").as_str() {
        "SHA-1" | "SHA-256" => 64,
        "SHA-384" | "SHA-512" => 128,
        _ => 0,
    }
}

fn is_secret_algorithm(name: &str) -> bool {
    name.starts_with("AES-") || name == "HMAC" || name == "HKDF" || name == "PBKDF2"
}

fn named_curve(ctx: &Ctx<'_>, algorithm: &Algorithm) -> JsResult<Nid> {
    match algorithm.named_curve.as_deref() {
        Some("P-256") => Ok(Nid::X9_62_PRIME256V1),
        Some("P-384") => Ok(Nid::SECP384R1),
        Some("P-521") => Ok(Nid::SECP521R1),
        _ => Err(throw(ctx.clone(), "unsupported named curve".to_string())),
    }
}

fn coordinate_size(ctx: &Ctx<'_>, algorithm: &Algorithm) -> JsResult<usize> {
    match algorithm.named_curve.as_deref() {
        Some("P-256") => Ok(32),
        Some("P-384") => Ok(48),
        Some("P-521") => Ok(66),
        _ => Err(throw(ctx.clone(), "unsupported named curve".to_string())),
    }
}

fn ecdsa_signature_to_raw(
    ctx: &Ctx<'_>,
    r: &openssl::bn::BigNumRef,
    s: &openssl::bn::BigNumRef,
    size: usize,
) -> JsResult<Vec<u8>> {
    let mut output = vec![0u8; size * 2];
    copy_bn(ctx, r, &mut output[..size])?;
    copy_bn(ctx, s, &mut output[size..])?;
    Ok(output)
}

fn copy_bn(ctx: &Ctx<'_>, value: &openssl::bn::BigNumRef, output: &mut [u8]) -> JsResult<()> {
    let bytes = value.to_vec();
    if bytes.len() > output.len() {
        return Err(throw(ctx.clone(), "integer too large".to_string()));
    }
    let start = output.len() - bytes.len();
    output[start..].copy_from_slice(&bytes);
    Ok(())
}

fn ecdsa_signature_from_raw(ctx: &Ctx<'_>, signature: &[u8], size: usize) -> JsResult<Vec<u8>> {
    if signature.len() != size * 2 {
        return Err(throw(ctx.clone(), "invalid ECDSA signature".to_string()));
    }
    let r = BigNum::from_slice(&signature[..size])
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    let s = BigNum::from_slice(&signature[size..])
        .map_err(|error| throw(ctx.clone(), error.to_string()))?;
    EcdsaSig::from_private_components(r, s)
        .and_then(|signature| signature.to_der())
        .map_err(|error| throw(ctx.clone(), error.to_string()))
}

fn bytes_from_value<'js>(ctx: &Ctx<'js>, value: Value<'js>) -> JsResult<Vec<u8>> {
    if let Some(buffer) = rquickjs::ArrayBuffer::from_value(value.clone()) {
        // SAFETY: copy immediately and don't call back into js while borrowed
        return unsafe {
            buffer
                .as_bytes()
                .map(|bytes| bytes.to_vec())
                .ok_or_else(|| Exception::throw_type(ctx, "buffer is detached"))
        };
    }

    if let Some(object) = value.as_object() {
        if let Some(buffer) = object.get::<_, Option<Value>>("buffer")? {
            if let Some(buffer) = rquickjs::ArrayBuffer::from_value(buffer) {
                let offset = object.get::<_, Option<usize>>("byteOffset")?.unwrap_or(0);
                let len = object
                    .get::<_, Option<usize>>("byteLength")?
                    .unwrap_or_else(|| buffer.len());
                // SAFETY: copy immediately and don't call back into js while borrowed
                if let Some(bytes) = unsafe { buffer.as_bytes() } {
                    let end = offset.saturating_add(len);
                    if end <= bytes.len() {
                        return Ok(bytes[offset..end].to_vec());
                    }
                }
            }
        }
    }

    Err(Exception::throw_type(
        ctx,
        "input must be an ArrayBuffer or ArrayBufferView",
    ))
}

fn throw(ctx: Ctx<'_>, message: String) -> rquickjs::Error {
    Exception::throw_message(&ctx, &message)
}

fn not_supported(ctx: &Ctx<'_>, operation: &str) -> rquickjs::Error {
    Exception::throw_message(ctx, &format!("{operation} is not supported"))
}

fn import_jwk<'js>(
    ctx: &Ctx<'js>,
    key_data: Value<'js>,
    algorithm: &Algorithm,
) -> JsResult<(&'static str, Vec<u8>)> {
    let object = key_data
        .into_object()
        .ok_or_else(|| Exception::throw_type(ctx, "JWK must be an object"))?;
    let kty = object.get::<_, String>("kty")?;

    match kty.as_str() {
        "oct" => {
            let k = object.get::<_, String>("k")?;
            let data = URL_SAFE_NO_PAD
                .decode(k)
                .map_err(|_| Exception::throw_type(ctx, "invalid JWK material"))?;
            Ok(("secret", data))
        }
        "RSA" => {
            let n = jwk_bn(&object, "n", ctx)?;
            let e = jwk_bn(&object, "e", ctx)?;

            if let Some(d) = object.get::<_, Option<String>>("d")? {
                let d = jwk_bn(&object, "d", ctx)?;
                let p = jwk_bn(&object, "p", ctx)?;
                let q = jwk_bn(&object, "q", ctx)?;
                let dp = jwk_bn(&object, "dp", ctx)?;
                let dq = jwk_bn(&object, "dq", ctx)?;
                let qi = jwk_bn(&object, "qi", ctx)?;
                let builder = openssl::rsa::RsaPrivateKeyBuilder::new(n, e, d)
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                let builder = builder
                    .set_factors(p, q)
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                let builder = builder
                    .set_crt_params(dp, dq, qi)
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                let key = PKey::from_rsa(builder.build())
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                Ok((
                    "private",
                    key.private_key_to_pkcs8()
                        .map_err(|error| throw(ctx.clone(), error.to_string()))?,
                ))
            } else {
                let key = openssl::rsa::Rsa::from_public_components(n, e)
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                let key =
                    PKey::from_rsa(key).map_err(|error| throw(ctx.clone(), error.to_string()))?;
                Ok((
                    "public",
                    key.public_key_to_der()
                        .map_err(|error| throw(ctx.clone(), error.to_string()))?,
                ))
            }
        }
        "EC" => {
            let curve = match object.get::<_, String>("crv")?.as_str() {
                "P-256" => Nid::X9_62_PRIME256V1,
                "P-384" => Nid::SECP384R1,
                "P-521" => Nid::SECP521R1,
                _ => return Err(not_supported(ctx, "EC curve")),
            };
            let x = jwk_bn(&object, "x", ctx)?;
            let y = jwk_bn(&object, "y", ctx)?;
            let group = EcGroup::from_curve_name(curve)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            let public_key = EcKey::from_public_key_affine_coordinates(&group, &x, &y)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;

            if let Some(d) = object.get::<_, Option<String>>("d")? {
                let d = URL_SAFE_NO_PAD
                    .decode(d)
                    .map_err(|_| Exception::throw_type(ctx, "invalid JWK material"))?;
                let d = BigNum::from_slice(&d)
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                let key = EcKey::from_private_components(&group, &d, public_key.public_key())
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                let key = PKey::from_ec_key(key)
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                Ok((
                    "private",
                    key.private_key_to_pkcs8()
                        .map_err(|error| throw(ctx.clone(), error.to_string()))?,
                ))
            } else {
                let key = PKey::from_ec_key(public_key)
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                Ok((
                    "public",
                    key.public_key_to_der()
                        .map_err(|error| throw(ctx.clone(), error.to_string()))?,
                ))
            }
        }
        "OKP" => {
            let crv = object.get::<_, String>("crv")?;
            let id = match crv.as_str() {
                "Ed25519" => Id::ED25519,
                "X25519" => Id::X25519,
                _ => return Err(not_supported(ctx, "OKP curve")),
            };
            if let Some(d) = object.get::<_, Option<String>>("d")? {
                let d = URL_SAFE_NO_PAD
                    .decode(d)
                    .map_err(|_| Exception::throw_type(ctx, "invalid JWK material"))?;
                let key = PKey::private_key_from_raw_bytes(&d, id)
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                Ok((
                    "private",
                    key.private_key_to_pkcs8()
                        .map_err(|error| throw(ctx.clone(), error.to_string()))?,
                ))
            } else {
                let x = object.get::<_, String>("x")?;
                let x = URL_SAFE_NO_PAD
                    .decode(x)
                    .map_err(|_| Exception::throw_type(ctx, "invalid JWK material"))?;
                let key = PKey::public_key_from_raw_bytes(&x, id)
                    .map_err(|error| throw(ctx.clone(), error.to_string()))?;
                Ok((
                    "public",
                    key.public_key_to_der()
                        .map_err(|error| throw(ctx.clone(), error.to_string()))?,
                ))
            }
        }
        _ => Err(not_supported(ctx, "JWK key type")),
    }
}

fn export_jwk<'js>(ctx: &Ctx<'js>, key: &KeyInput) -> JsResult<Value<'js>> {
    let name = key.algorithm.name.to_ascii_uppercase();
    let object = Object::new(ctx.clone())?;
    if is_secret_algorithm(&name) {
        object.set("kty", "oct")?;
        object.set("k", URL_SAFE_NO_PAD.encode(&key.data))?;
        return Ok(object.into_value());
    }

    if name.starts_with("RSA") {
        if key.key_type == "private" {
            let pkey = private_pkey(ctx, key)?;
            let rsa = pkey
                .rsa()
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            object.set("kty", "RSA")?;
            set_bn_jwk(&object, "n", rsa.n())?;
            set_bn_jwk(&object, "e", rsa.e())?;
            set_bn_jwk(&object, "d", rsa.d())?;
            if let Some(p) = rsa.p() {
                set_bn_jwk(&object, "p", p)?;
            }
            if let Some(q) = rsa.q() {
                set_bn_jwk(&object, "q", q)?;
            }
            if let Some(dp) = rsa.dmp1() {
                set_bn_jwk(&object, "dp", dp)?;
            }
            if let Some(dq) = rsa.dmq1() {
                set_bn_jwk(&object, "dq", dq)?;
            }
            if let Some(qi) = rsa.iqmp() {
                set_bn_jwk(&object, "qi", qi)?;
            }
        } else {
            let pkey = public_pkey(ctx, key)?;
            let rsa = pkey
                .rsa()
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            object.set("kty", "RSA")?;
            set_bn_jwk(&object, "n", rsa.n())?;
            set_bn_jwk(&object, "e", rsa.e())?;
        }
        return Ok(object.into_value());
    }

    if name == "ECDSA" || name == "ECDH" {
        object.set("kty", "EC")?;
        object.set("crv", key.algorithm.named_curve.as_deref().unwrap_or(""))?;

        if key.key_type == "private" {
            let pkey = private_pkey(ctx, key)?;
            let ec = pkey
                .ec_key()
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            let mut context =
                BigNumContext::new().map_err(|error| throw(ctx.clone(), error.to_string()))?;
            let mut x = BigNum::new().map_err(|error| throw(ctx.clone(), error.to_string()))?;
            let mut y = BigNum::new().map_err(|error| throw(ctx.clone(), error.to_string()))?;
            ec.public_key()
                .affine_coordinates_gfp(ec.group(), &mut x, &mut y, &mut context)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            set_bn_jwk(&object, "x", &x)?;
            set_bn_jwk(&object, "y", &y)?;
            set_bn_jwk(&object, "d", ec.private_key())?;
        } else {
            let pkey = public_pkey(ctx, key)?;
            let ec = pkey
                .ec_key()
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            let mut context =
                BigNumContext::new().map_err(|error| throw(ctx.clone(), error.to_string()))?;
            let mut x = BigNum::new().map_err(|error| throw(ctx.clone(), error.to_string()))?;
            let mut y = BigNum::new().map_err(|error| throw(ctx.clone(), error.to_string()))?;
            ec.public_key()
                .affine_coordinates_gfp(ec.group(), &mut x, &mut y, &mut context)
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            set_bn_jwk(&object, "x", &x)?;
            set_bn_jwk(&object, "y", &y)?;
        }
        return Ok(object.into_value());
    }

    if name == "ED25519" || name == "X25519" {
        object.set("kty", "OKP")?;
        object.set(
            "crv",
            if name == "ED25519" {
                "Ed25519"
            } else {
                "X25519"
            },
        )?;
        if key.key_type == "private" {
            let pkey = private_pkey(ctx, key)?;
            let public = pkey
                .raw_public_key()
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            object.set("x", URL_SAFE_NO_PAD.encode(public))?;
            let private = pkey
                .raw_private_key()
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            object.set("d", URL_SAFE_NO_PAD.encode(private))?;
        } else {
            let pkey = public_pkey(ctx, key)?;
            let public = pkey
                .raw_public_key()
                .map_err(|error| throw(ctx.clone(), error.to_string()))?;
            object.set("x", URL_SAFE_NO_PAD.encode(public))?;
        }
        return Ok(object.into_value());
    }

    Err(not_supported(ctx, "JWK export"))
}

fn set_bn_jwk(object: &Object<'_>, name: &str, value: &openssl::bn::BigNumRef) -> JsResult<()> {
    object.set(name, URL_SAFE_NO_PAD.encode(value.to_vec()))?;
    Ok(())
}

fn jwk_bn(object: &Object<'_>, name: &str, ctx: &Ctx<'_>) -> JsResult<BigNum> {
    let value = object.get::<_, String>(name)?;
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| Exception::throw_type(ctx, "invalid JWK material"))?;
    BigNum::from_slice(&bytes).map_err(|error| throw(ctx.clone(), error.to_string()))
}

#[cfg(test)]
mod tests {
    use rquickjs::{Context, Promise, Runtime};

    use super::setup;

    fn run(source: &str) -> String {
        let rt = Runtime::new().unwrap();
        let ctx = Context::full(&rt).unwrap();

        ctx.with(|ctx| {
            setup(&ctx).unwrap();
            let promise = ctx.eval::<Promise, _>(source).unwrap();
            match promise.finish::<String>() {
                Ok(value) => value,
                Err(_) => panic!("javascript error: {:?}", ctx.catch()),
            }
        })
    }

    #[test]
    fn rsa_algorithms() {
        let result = run(r#"
                (async () => {
                    const oaep = await crypto.subtle.generateKey(
                        {
                            name: "RSA-OAEP",
                            modulusLength: 1024,
                            publicExponent: new Uint8Array([1, 0, 1]),
                            hash: "SHA-256",
                        },
                        true,
                        ["encrypt", "decrypt"],
                    );
                    const ciphertext = await crypto.subtle.encrypt(
                        { name: "RSA-OAEP" },
                        oaep.publicKey,
                        new Uint8Array([1, 2, 3]),
                    );
                    const plaintext = await crypto.subtle.decrypt(
                        { name: "RSA-OAEP" },
                        oaep.privateKey,
                        ciphertext,
                    );

                    const pss = await crypto.subtle.generateKey(
                        {
                            name: "RSA-PSS",
                            modulusLength: 1024,
                            publicExponent: new Uint8Array([1, 0, 1]),
                            hash: "SHA-256",
                        },
                        true,
                        ["sign", "verify"],
                    );
                    const signature = await crypto.subtle.sign(
                        { name: "RSA-PSS", saltLength: 32 },
                        pss.privateKey,
                        new Uint8Array([4, 5, 6]),
                    );
                    const valid = await crypto.subtle.verify(
                        { name: "RSA-PSS", saltLength: 32 },
                        pss.publicKey,
                        signature,
                        new Uint8Array([4, 5, 6]),
                    );
                    const jwk = await crypto.subtle.exportKey("jwk", pss.privateKey);
                    const imported = await crypto.subtle.importKey(
                        "jwk",
                        jwk,
                        { name: "RSA-PSS", hash: "SHA-256" },
                        true,
                        ["sign"],
                    );
                    const signature2 = await crypto.subtle.sign(
                        { name: "RSA-PSS", saltLength: 32 },
                        imported,
                        new Uint8Array([4, 5, 6]),
                    );
                    const valid2 = await crypto.subtle.verify(
                        { name: "RSA-PSS", saltLength: 32 },
                        pss.publicKey,
                        signature2,
                        new Uint8Array([4, 5, 6]),
                    );
                    return [
                        Array.from(new Uint8Array(plaintext)).join(""),
                        valid,
                        valid2,
                        jwk.kty,
                    ].join(",");
                })()
            "#);
        assert_eq!(result, "123,true,true,RSA");
    }

    #[test]
    fn ec_and_okp_algorithms() {
        let result = run(r#"
                (async () => {
                    const ecdsa = await crypto.subtle.generateKey(
                        { name: "ECDSA", namedCurve: "P-256" },
                        true,
                        ["sign", "verify"],
                    );
                    const signature = await crypto.subtle.sign(
                        { name: "ECDSA", hash: "SHA-256" },
                        ecdsa.privateKey,
                        new Uint8Array([1, 2, 3]),
                    );
                    const valid = await crypto.subtle.verify(
                        { name: "ECDSA", hash: "SHA-256" },
                        ecdsa.publicKey,
                        signature,
                        new Uint8Array([1, 2, 3]),
                    );
                    const jwk = await crypto.subtle.exportKey("jwk", ecdsa.privateKey);
                    const imported = await crypto.subtle.importKey(
                        "jwk",
                        jwk,
                        { name: "ECDSA", namedCurve: "P-256" },
                        true,
                        ["sign"],
                    );
                    const signature2 = await crypto.subtle.sign(
                        { name: "ECDSA", hash: "SHA-256" },
                        imported,
                        new Uint8Array([1, 2, 3]),
                    );
                    const valid2 = await crypto.subtle.verify(
                        { name: "ECDSA", hash: "SHA-256" },
                        ecdsa.publicKey,
                        signature2,
                        new Uint8Array([1, 2, 3]),
                    );

                    const alice = await crypto.subtle.generateKey(
                        { name: "ECDH", namedCurve: "P-256" },
                        true,
                        ["deriveBits"],
                    );
                    const bob = await crypto.subtle.generateKey(
                        { name: "ECDH", namedCurve: "P-256" },
                        true,
                        ["deriveBits"],
                    );
                    const aliceSecret = await crypto.subtle.deriveBits(
                        { name: "ECDH", public: bob.publicKey },
                        alice.privateKey,
                        256,
                    );
                    const bobSecret = await crypto.subtle.deriveBits(
                        { name: "ECDH", public: alice.publicKey },
                        bob.privateKey,
                        256,
                    );

                    const ed = await crypto.subtle.generateKey(
                        { name: "Ed25519" },
                        true,
                        ["sign", "verify"],
                    );
                    const edSignature = await crypto.subtle.sign(
                        { name: "Ed25519" },
                        ed.privateKey,
                        new Uint8Array([7, 8, 9]),
                    );
                    const edValid = await crypto.subtle.verify(
                        { name: "Ed25519" },
                        ed.publicKey,
                        edSignature,
                        new Uint8Array([7, 8, 9]),
                    );

                    const x1 = await crypto.subtle.generateKey(
                        { name: "X25519" },
                        true,
                        ["deriveBits"],
                    );
                    const x2 = await crypto.subtle.generateKey(
                        { name: "X25519" },
                        true,
                        ["deriveBits"],
                    );
                    const x1Secret = await crypto.subtle.deriveBits(
                        { name: "X25519", public: x2.publicKey },
                        x1.privateKey,
                        256,
                    );
                    const x2Secret = await crypto.subtle.deriveBits(
                        { name: "X25519", public: x1.publicKey },
                        x2.privateKey,
                        256,
                    );

                    return [
                        valid,
                        valid2,
                        jwk.kty,
                        new Uint8Array(aliceSecret).join(",") === new Uint8Array(bobSecret).join(","),
                        edValid,
                        new Uint8Array(x1Secret).join(",") === new Uint8Array(x2Secret).join(","),
                    ].join(",");
                })()
            "#);
        assert_eq!(result, "true,true,EC,true,true,true");
    }

    #[test]
    fn digest_and_hmac() {
        let result = run(r#"
                (async () => {
                    const digest = await crypto.subtle.digest("SHA-256", new Uint8Array([97, 98, 99]));
                    const key = await crypto.subtle.generateKey(
                        { name: "HMAC", hash: "SHA-256" },
                        true,
                        ["sign", "verify"],
                    );
                    const signature = await crypto.subtle.sign("HMAC", key, new Uint8Array([1, 2, 3]));
                    const valid = await crypto.subtle.verify("HMAC", key, signature, new Uint8Array([1, 2, 3]));
                    return [digest.byteLength, valid].join(",");
                })()
            "#);
        assert_eq!(result, "32,true");
    }

    #[test]
    fn aes_and_kdf() {
        let result = run(r#"
                (async () => {
                    const key = await crypto.subtle.generateKey(
                        { name: "AES-GCM", length: 256 },
                        true,
                        ["encrypt", "decrypt"],
                    );
                    const iv = new Uint8Array(12);
                    const ciphertext = await crypto.subtle.encrypt(
                        { name: "AES-GCM", iv },
                        key,
                        new Uint8Array([104, 101, 108, 108, 111]),
                    );
                    const plaintext = await crypto.subtle.decrypt({ name: "AES-GCM", iv }, key, ciphertext);
                    const base = await crypto.subtle.importKey(
                        "raw",
                        new Uint8Array(32),
                        "PBKDF2",
                        false,
                        ["deriveBits"],
                    );
                    const bits = await crypto.subtle.deriveBits(
                        { name: "PBKDF2", salt: new Uint8Array(16), iterations: 1, hash: "SHA-256" },
                        base,
                        128,
                    );

                    const kek = await crypto.subtle.generateKey(
                        { name: "AES-KW", length: 256 },
                        true,
                        ["wrapKey", "unwrapKey"],
                    );
                    const wrapped = await crypto.subtle.wrapKey("raw", key, kek, "AES-KW");
                    const unwrapped = await crypto.subtle.unwrapKey(
                        "raw",
                        wrapped,
                        kek,
                        "AES-KW",
                        { name: "AES-GCM", length: 256 },
                        true,
                        ["encrypt", "decrypt"],
                    );
                    const originalRaw = new Uint8Array(await crypto.subtle.exportKey("raw", key));
                    const unwrappedRaw = new Uint8Array(await crypto.subtle.exportKey("raw", unwrapped));
                    const sameKey =
                        originalRaw.join(",") === unwrappedRaw.join(",");

                    return [
                        Array.from(new Uint8Array(plaintext)).join(""),
                        bits.byteLength,
                        sameKey,
                    ].join(",");
                })()
            "#);
        assert_eq!(result, "104101108108111,16,true");
    }
}
