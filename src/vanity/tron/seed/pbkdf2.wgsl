const RECORD_WORDS: u32 = 57u;
const RECORD_DATA_BYTES: u32 = 224u;

const SHA512_K: array<vec2<u32>, 80> = array<vec2<u32>, 80>(
    vec2<u32>(0x428a2f98u, 0xd728ae22u), vec2<u32>(0x71374491u, 0x23ef65cdu),
    vec2<u32>(0xb5c0fbcfu, 0xec4d3b2fu), vec2<u32>(0xe9b5dba5u, 0x8189dbbcu),
    vec2<u32>(0x3956c25bu, 0xf348b538u), vec2<u32>(0x59f111f1u, 0xb605d019u),
    vec2<u32>(0x923f82a4u, 0xaf194f9bu), vec2<u32>(0xab1c5ed5u, 0xda6d8118u),
    vec2<u32>(0xd807aa98u, 0xa3030242u), vec2<u32>(0x12835b01u, 0x45706fbeu),
    vec2<u32>(0x243185beu, 0x4ee4b28cu), vec2<u32>(0x550c7dc3u, 0xd5ffb4e2u),
    vec2<u32>(0x72be5d74u, 0xf27b896fu), vec2<u32>(0x80deb1feu, 0x3b1696b1u),
    vec2<u32>(0x9bdc06a7u, 0x25c71235u), vec2<u32>(0xc19bf174u, 0xcf692694u),
    vec2<u32>(0xe49b69c1u, 0x9ef14ad2u), vec2<u32>(0xefbe4786u, 0x384f25e3u),
    vec2<u32>(0x0fc19dc6u, 0x8b8cd5b5u), vec2<u32>(0x240ca1ccu, 0x77ac9c65u),
    vec2<u32>(0x2de92c6fu, 0x592b0275u), vec2<u32>(0x4a7484aau, 0x6ea6e483u),
    vec2<u32>(0x5cb0a9dcu, 0xbd41fbd4u), vec2<u32>(0x76f988dau, 0x831153b5u),
    vec2<u32>(0x983e5152u, 0xee66dfabu), vec2<u32>(0xa831c66du, 0x2db43210u),
    vec2<u32>(0xb00327c8u, 0x98fb213fu), vec2<u32>(0xbf597fc7u, 0xbeef0ee4u),
    vec2<u32>(0xc6e00bf3u, 0x3da88fc2u), vec2<u32>(0xd5a79147u, 0x930aa725u),
    vec2<u32>(0x06ca6351u, 0xe003826fu), vec2<u32>(0x14292967u, 0x0a0e6e70u),
    vec2<u32>(0x27b70a85u, 0x46d22ffcu), vec2<u32>(0x2e1b2138u, 0x5c26c926u),
    vec2<u32>(0x4d2c6dfcu, 0x5ac42aedu), vec2<u32>(0x53380d13u, 0x9d95b3dfu),
    vec2<u32>(0x650a7354u, 0x8baf63deu), vec2<u32>(0x766a0abbu, 0x3c77b2a8u),
    vec2<u32>(0x81c2c92eu, 0x47edaee6u), vec2<u32>(0x92722c85u, 0x1482353bu),
    vec2<u32>(0xa2bfe8a1u, 0x4cf10364u), vec2<u32>(0xa81a664bu, 0xbc423001u),
    vec2<u32>(0xc24b8b70u, 0xd0f89791u), vec2<u32>(0xc76c51a3u, 0x0654be30u),
    vec2<u32>(0xd192e819u, 0xd6ef5218u), vec2<u32>(0xd6990624u, 0x5565a910u),
    vec2<u32>(0xf40e3585u, 0x5771202au), vec2<u32>(0x106aa070u, 0x32bbd1b8u),
    vec2<u32>(0x19a4c116u, 0xb8d2d0c8u), vec2<u32>(0x1e376c08u, 0x5141ab53u),
    vec2<u32>(0x2748774cu, 0xdf8eeb99u), vec2<u32>(0x34b0bcb5u, 0xe19b48a8u),
    vec2<u32>(0x391c0cb3u, 0xc5c95a63u), vec2<u32>(0x4ed8aa4au, 0xe3418acbu),
    vec2<u32>(0x5b9cca4fu, 0x7763e373u), vec2<u32>(0x682e6ff3u, 0xd6b2b8a3u),
    vec2<u32>(0x748f82eeu, 0x5defb2fcu), vec2<u32>(0x78a5636fu, 0x43172f60u),
    vec2<u32>(0x84c87814u, 0xa1f0ab72u), vec2<u32>(0x8cc70208u, 0x1a6439ecu),
    vec2<u32>(0x90befffau, 0x23631e28u), vec2<u32>(0xa4506cebu, 0xde82bde9u),
    vec2<u32>(0xbef9a3f7u, 0xb2c67915u), vec2<u32>(0xc67178f2u, 0xe372532bu),
    vec2<u32>(0xca273eceu, 0xea26619cu), vec2<u32>(0xd186b8c7u, 0x21c0c207u),
    vec2<u32>(0xeada7dd6u, 0xcde0eb1eu), vec2<u32>(0xf57d4f7fu, 0xee6ed178u),
    vec2<u32>(0x06f067aau, 0x72176fbau), vec2<u32>(0x0a637dc5u, 0xa2c898a6u),
    vec2<u32>(0x113f9804u, 0xbef90daeu), vec2<u32>(0x1b710b35u, 0x131c471bu),
    vec2<u32>(0x28db77f5u, 0x23047d84u), vec2<u32>(0x32caab7bu, 0x40c72493u),
    vec2<u32>(0x3c9ebe0au, 0x15c9bebcu), vec2<u32>(0x431d67c4u, 0x9c100d4cu),
    vec2<u32>(0x4cc5d4beu, 0xcb3e42b6u), vec2<u32>(0x597f299cu, 0xfc657e2au),
    vec2<u32>(0x5fcb6fabu, 0x3ad6faecu), vec2<u32>(0x6c44198cu, 0x4a475817u),
);

@group(0) @binding(0) var<storage, read> input_words: array<u32>;
@group(0) @binding(1) var<storage, read_write> output_words: array<u32>;

fn join_u64(parts: vec2<u32>) -> u64 {
    return (u64(parts.x) << 32u) | u64(parts.y);
}

fn initial_state() -> array<u64, 8> {
    return array<u64, 8>(
        join_u64(vec2<u32>(0x6a09e667u, 0xf3bcc908u)),
        join_u64(vec2<u32>(0xbb67ae85u, 0x84caa73bu)),
        join_u64(vec2<u32>(0x3c6ef372u, 0xfe94f82bu)),
        join_u64(vec2<u32>(0xa54ff53au, 0x5f1d36f1u)),
        join_u64(vec2<u32>(0x510e527fu, 0xade682d1u)),
        join_u64(vec2<u32>(0x9b05688cu, 0x2b3e6c1fu)),
        join_u64(vec2<u32>(0x1f83d9abu, 0xfb41bd6bu)),
        join_u64(vec2<u32>(0x5be0cd19u, 0x137e2179u)),
    );
}

fn rotr(value: u64, amount: u32) -> u64 {
    return (value >> amount) | (value << (64u - amount));
}

fn big_sigma0(value: u64) -> u64 {
    return rotr(value, 28u) ^ rotr(value, 34u) ^ rotr(value, 39u);
}

fn big_sigma1(value: u64) -> u64 {
    return rotr(value, 14u) ^ rotr(value, 18u) ^ rotr(value, 41u);
}

fn small_sigma0(value: u64) -> u64 {
    return rotr(value, 1u) ^ rotr(value, 8u) ^ (value >> 7u);
}

fn small_sigma1(value: u64) -> u64 {
    return rotr(value, 19u) ^ rotr(value, 61u) ^ (value >> 6u);
}

fn sha512_compress(state: array<u64, 8>, block: array<u64, 16>) -> array<u64, 8> {
    var schedule = block;
    var a = state[0];
    var b = state[1];
    var c = state[2];
    var d = state[3];
    var e = state[4];
    var f = state[5];
    var g = state[6];
    var h = state[7];

    for (var round = 0u; round < 80u; round = round + 1u) {
        let slot = round & 15u;
        if (round >= 16u) {
            let word_2 = schedule[(round - 2u) & 15u];
            let word_7 = schedule[(round - 7u) & 15u];
            let word_15 = schedule[(round - 15u) & 15u];
            let word_16 = schedule[slot];
            schedule[slot] = small_sigma1(word_2)
                + word_7
                + small_sigma0(word_15)
                + word_16;
        }
        let choose = (e & f) ^ ((~e) & g);
        let majority = (a & b) ^ (a & c) ^ (b & c);
        let temp1 = h + big_sigma1(e) + choose + join_u64(SHA512_K[round]) + schedule[slot];
        let temp2 = big_sigma0(a) + majority;
        h = g;
        g = f;
        f = e;
        e = d + temp1;
        d = c;
        c = b;
        b = a;
        a = temp1 + temp2;
    }

    return array<u64, 8>(
        state[0] + a, state[1] + b, state[2] + c, state[3] + d,
        state[4] + e, state[5] + f, state[6] + g, state[7] + h,
    );
}

fn phrase_byte(record_base: u32, index: u32) -> u32 {
    let packed = input_words[record_base + 1u + index / 4u];
    return (packed >> ((index & 3u) * 8u)) & 0xffu;
}

fn phrase_word(record_base: u32, offset: u32, length: u32) -> u64 {
    var value = u64(0u);
    for (var byte_index = 0u; byte_index < 8u; byte_index = byte_index + 1u) {
        let source_index = offset + byte_index;
        var byte_value = 0u;
        if (source_index < length) {
            byte_value = phrase_byte(record_base, source_index);
        }
        value = value | (u64(byte_value) << ((7u - byte_index) * 8u));
    }
    return value;
}

fn set_block_byte(block: ptr<function, array<u64, 16>>, index: u32, value: u32) {
    let word_index = index / 8u;
    let shift = (7u - (index & 7u)) * 8u;
    (*block)[word_index] = (*block)[word_index] | (u64(value) << shift);
}

fn hash_long_phrase(record_base: u32, length: u32) -> array<u64, 8> {
    var first_block: array<u64, 16>;
    for (var word_index = 0u; word_index < 16u; word_index = word_index + 1u) {
        first_block[word_index] = phrase_word(record_base, word_index * 8u, length);
    }
    var state = sha512_compress(initial_state(), first_block);

    var final_block: array<u64, 16>;
    let remaining = length - 128u;
    for (var byte_index = 0u; byte_index < remaining; byte_index = byte_index + 1u) {
        set_block_byte(&final_block, byte_index, phrase_byte(record_base, 128u + byte_index));
    }
    set_block_byte(&final_block, remaining, 0x80u);
    final_block[15] = u64(length) * u64(8u);
    state = sha512_compress(state, final_block);
    return state;
}

struct HmacStates {
    inner: array<u64, 8>,
    outer: array<u64, 8>,
}

fn hmac_states(record_base: u32, length: u32) -> HmacStates {
    var inner_block: array<u64, 16>;
    var outer_block: array<u64, 16>;
    if (length > 128u) {
        let key_hash = hash_long_phrase(record_base, length);
        for (var index = 0u; index < 16u; index = index + 1u) {
            var key_word = u64(0u);
            if (index < 8u) {
                key_word = key_hash[index];
            }
            inner_block[index] = key_word ^ join_u64(vec2<u32>(0x36363636u, 0x36363636u));
            outer_block[index] = key_word ^ join_u64(vec2<u32>(0x5c5c5c5cu, 0x5c5c5c5cu));
        }
    } else {
        for (var index = 0u; index < 16u; index = index + 1u) {
            let key_word = phrase_word(record_base, index * 8u, length);
            inner_block[index] = key_word ^ join_u64(vec2<u32>(0x36363636u, 0x36363636u));
            outer_block[index] = key_word ^ join_u64(vec2<u32>(0x5c5c5c5cu, 0x5c5c5c5cu));
        }
    }
    return HmacStates(
        sha512_compress(initial_state(), inner_block),
        sha512_compress(initial_state(), outer_block),
    );
}

fn finish_hmac(states: HmacStates, message_block: array<u64, 16>) -> array<u64, 8> {
    let inner_digest = sha512_compress(states.inner, message_block);
    var outer_message: array<u64, 16>;
    for (var index = 0u; index < 8u; index = index + 1u) {
        outer_message[index] = inner_digest[index];
    }
    outer_message[8] = join_u64(vec2<u32>(0x80000000u, 0u));
    outer_message[15] = u64(1536u);
    return sha512_compress(states.outer, outer_message);
}

fn hmac_u1(states: HmacStates) -> array<u64, 8> {
    var message: array<u64, 16>;
    message[0] = join_u64(vec2<u32>(0x6d6e656du, 0x6f6e6963u));
    message[1] = join_u64(vec2<u32>(0x00000001u, 0x80000000u));
    message[15] = u64(1120u);
    return finish_hmac(states, message);
}

fn hmac_digest(states: HmacStates, digest: array<u64, 8>) -> array<u64, 8> {
    var message: array<u64, 16>;
    for (var index = 0u; index < 8u; index = index + 1u) {
        message[index] = digest[index];
    }
    message[8] = join_u64(vec2<u32>(0x80000000u, 0u));
    message[15] = u64(1536u);
    return finish_hmac(states, message);
}

fn pbkdf2_seed(record_base: u32, length: u32) -> array<u64, 8> {
    let states = hmac_states(record_base, length);
    var current = hmac_u1(states);
    var result = current;
    for (var iteration = 1u; iteration < 2048u; iteration = iteration + 1u) {
        current = hmac_digest(states, current);
        for (var word_index = 0u; word_index < 8u; word_index = word_index + 1u) {
            result[word_index] = result[word_index] ^ current[word_index];
        }
    }
    return result;
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let candidate_index = global_id.x;
    if (candidate_index >= input_words[0]) {
        return;
    }
    let record_base = 4u + candidate_index * RECORD_WORDS;
    let length = input_words[record_base];
    if (length > RECORD_DATA_BYTES) {
        return;
    }

    let result = pbkdf2_seed(record_base, length);

    let output_base = candidate_index * 16u;
    for (var word_index = 0u; word_index < 8u; word_index = word_index + 1u) {
        output_words[output_base + word_index * 2u] = u32(result[word_index] >> 32u);
        output_words[output_base + word_index * 2u + 1u] = u32(result[word_index]);
    }
}
