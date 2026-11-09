(() => {
    const bridge = globalThis.__lampreyCryptoBridge;
    delete globalThis.__lampreyCryptoBridge;

    const INTERNAL = Symbol("Crypto");
    const KEY_DATA = Symbol("CryptoKeyData");

    function notSupported(operation) {
        const error = new Error(`${operation} is not supported`);
        error.name = "NotSupportedError";
        return error;
    }

    function promise(operation) {
        try {
            return Promise.resolve(operation());
        } catch (error) {
            return Promise.reject(error);
        }
    }

    function toBytes(value) {
        if (value instanceof ArrayBuffer) {
            return new Uint8Array(value);
        }
        if (ArrayBuffer.isView(value)) {
            return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
        }
        throw new TypeError("expected an ArrayBuffer or ArrayBufferView");
    }

    function keyEntry(key) {
        if (!(key instanceof CryptoKey)) {
            throw new TypeError("expected a CryptoKey");
        }
        return key[KEY_DATA]();
    }

    function normalizeAlgorithm(algorithm) {
        if (
            algorithm &&
            typeof algorithm === "object" &&
            algorithm.public instanceof CryptoKey
        ) {
            return { ...algorithm, public: keyEntry(algorithm.public) };
        }
        return algorithm;
    }

    function wrapKey(entry, algorithm) {
        return new CryptoKey(INTERNAL, {
            type: entry.type,
            extractable: entry.extractable,
            algorithm,
            usages: entry.usages,
            data: entry.data,
        });
    }

    class CryptoKey {
        #algorithm;
        #extractable;
        #type;
        #usages;
        #data;

        constructor(token, init) {
            if (token !== INTERNAL) {
                throw new TypeError("Illegal constructor");
            }
            this.#algorithm = init.algorithm;
            this.#extractable = init.extractable;
            this.#type = init.type;
            this.#usages = init.usages;
            this.#data = init.data;
        }

        get algorithm() {
            return this.#algorithm;
        }

        get extractable() {
            return this.#extractable;
        }

        get type() {
            return this.#type;
        }

        get usages() {
            return this.#usages.slice();
        }

        get [Symbol.toStringTag]() {
            return "CryptoKey";
        }

        [KEY_DATA]() {
            return {
                type: this.#type,
                extractable: this.#extractable,
                algorithm: this.#algorithm,
                usages: this.#usages,
                data: this.#data,
            };
        }
    }

    class SubtleCrypto {
        constructor(token) {
            if (token !== INTERNAL) {
                throw new TypeError("Illegal constructor");
            }
        }

        encrypt(algorithm, key, data) {
            return promise(() => bridge.encrypt(algorithm, keyEntry(key), toBytes(data)));
        }

        decrypt(algorithm, key, data) {
            return promise(() => bridge.decrypt(algorithm, keyEntry(key), toBytes(data)));
        }

        sign(algorithm, key, data) {
            return promise(() => bridge.sign(algorithm, keyEntry(key), toBytes(data)));
        }

        verify(algorithm, key, signature, data) {
            return promise(() =>
                bridge.verify(algorithm, keyEntry(key), toBytes(signature), toBytes(data)),
            );
        }

        digest(algorithm, data) {
            return promise(() => bridge.digest(algorithm, toBytes(data)));
        }

        generateKey(algorithm, extractable, keyUsages) {
            return promise(() => {
                const result = bridge.generateKey(algorithm, extractable, keyUsages);
                if (result.privateKey || result.publicKey) {
                    return {
                        privateKey: wrapKey(result.privateKey, algorithm),
                        publicKey: wrapKey(result.publicKey, algorithm),
                    };
                }
                return wrapKey(result, algorithm);
            });
        }

        deriveKey(algorithm, baseKey, derivedKeyType, extractable, keyUsages) {
            return promise(() => {
                const length = derivedKeyType.length ?? 256;
                const bits = bridge.deriveBits(normalizeAlgorithm(algorithm), keyEntry(baseKey), length);
                const key = bridge.importKey(
                    "raw",
                    bits,
                    derivedKeyType,
                    extractable,
                    keyUsages,
                );
                return wrapKey(key, derivedKeyType);
            });
        }

        deriveBits(algorithm, baseKey, length) {
            return promise(() =>
                bridge.deriveBits(normalizeAlgorithm(algorithm), keyEntry(baseKey), length),
            );
        }

        importKey(format, keyData, algorithm, extractable, keyUsages) {
            return promise(() => {
                const data = format === "jwk" ? keyData : toBytes(keyData);
                const key = bridge.importKey(format, data, algorithm, extractable, keyUsages);
                return wrapKey(key, algorithm);
            });
        }

        exportKey(format, key) {
            return promise(() => {
                const result = bridge.exportKey(format, keyEntry(key));
                return format === "jwk" ? result : toBytes(result);
            });
        }

        async wrapKey(format, key, wrappingKey, wrapAlgorithm) {
            const data =
                format === "jwk"
                    ? new TextEncoder().encode(
                          JSON.stringify(await this.exportKey("jwk", key)),
                      )
                    : keyEntry(key).data;
            return bridge.wrapKey(wrapAlgorithm, keyEntry(wrappingKey), data);
        }

        unwrapKey(
            format,
            wrappedKey,
            unwrappingKey,
            unwrapAlgorithm,
            unwrappedKeyAlgorithm,
            extractable,
            keyUsages,
        ) {
            return promise(() => {
                const data = bridge.unwrapKey(
                    unwrapAlgorithm,
                    keyEntry(unwrappingKey),
                    toBytes(wrappedKey),
                );
                const key = bridge.importKey(
                    format,
                    format === "jwk" ? JSON.parse(new TextDecoder().decode(data)) : data,
                    unwrappedKeyAlgorithm,
                    extractable,
                    keyUsages,
                );
                return wrapKey(key, unwrappedKeyAlgorithm);
            });
        }

        get [Symbol.toStringTag]() {
            return "SubtleCrypto";
        }
    }

    class Crypto {
        #subtle = new SubtleCrypto(INTERNAL);

        constructor(token) {
            if (token !== INTERNAL) {
                throw new TypeError("Illegal constructor");
            }
        }

        get subtle() {
            return this.#subtle;
        }

        getRandomValues(array) {
            if (!ArrayBuffer.isView(array)) {
                throw new TypeError("expected an ArrayBufferView");
            }
            if (array.byteLength > 65536) {
                throw new RangeError("requested length is too large");
            }
            const bytes = bridge.randomBytes(array.byteLength);
            new Uint8Array(array.buffer, array.byteOffset, array.byteLength).set(bytes);
            return array;
        }

        randomUUID() {
            const bytes = bridge.randomBytes(16);
            bytes[6] = (bytes[6] & 0x0f) | 0x40;
            bytes[8] = (bytes[8] & 0x3f) | 0x80;
            const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
            return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
        }

        get [Symbol.toStringTag]() {
            return "Crypto";
        }
    }

    Object.assign(globalThis, {
        Crypto,
        CryptoKey,
        SubtleCrypto,
        crypto: new Crypto(INTERNAL),
    });
})();
