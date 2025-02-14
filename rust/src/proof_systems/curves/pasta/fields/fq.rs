use ark_ff::{
    biginteger::BigInteger256 as BigInteger, FftParameters, Fp256, Fp256Parameters, FpParameters,
};

pub struct FqParameters;

pub type Fq = Fp256<FqParameters>;

impl Fp256Parameters for FqParameters {}

impl FftParameters for FqParameters {
    type BigInt = BigInteger;

    const TWO_ADICITY: u32 = 32;

    #[rustfmt::skip]
    const TWO_ADIC_ROOT_OF_UNITY: BigInteger = BigInteger([
        0x218077428c9942de, 0xcc49578921b60494, 0xac2e5d27b2efbee2, 0xb79fa897f2db056
    ]);
}

impl FpParameters for FqParameters {
    // 28948022309329048855892746252171976963363056481941647379679742748393362948097
    const MODULUS: BigInteger = BigInteger([
        0x8c46eb2100000001,
        0x224698fc0994a8dd,
        0x0,
        0x4000000000000000,
    ]);

    const MODULUS_BITS: u32 = 255;

    const REPR_SHAVE_BITS: u32 = 1;

    const R: BigInteger = BigInteger([
        0x5b2b3e9cfffffffd,
        0x992c350be3420567,
        0xffffffffffffffff,
        0x3fffffffffffffff,
    ]);

    // T and T_MINUS_ONE_DIV_TWO, where MODULUS - 1 = 2^S * T

    const R2: BigInteger = BigInteger([
        0xfc9678ff0000000f,
        0x67bb433d891a16e3,
        0x7fae231004ccf590,
        0x96d41af7ccfdaa9,
    ]);

    // -(MODULUS^{-1} mod 2^64) mod 2^64
    const INV: u64 = 10108024940646105087;

    // GENERATOR = 5
    const GENERATOR: BigInteger = BigInteger([
        0x96bc8c8cffffffed,
        0x74c2a54b49f7778e,
        0xfffffffffffffffd,
        0x3fffffffffffffff,
    ]);

    const CAPACITY: u32 = Self::MODULUS_BITS - 1;

    const T: BigInteger = BigInteger([0x994a8dd8c46eb21, 0x224698fc, 0x0, 0x40000000]);

    const T_MINUS_ONE_DIV_TWO: BigInteger =
        BigInteger([0x4ca546ec6237590, 0x11234c7e, 0x0, 0x20000000]);

    const MODULUS_MINUS_ONE_DIV_TWO: BigInteger = BigInteger([
        0xc623759080000000,
        0x11234c7e04ca546e,
        0x0,
        0x2000000000000000,
    ]);
}
