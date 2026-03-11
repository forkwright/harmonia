// JNI bindings for Android

#[cfg(feature = "android")]
use jni::objects::{JByteArray, JClass, JObject};
#[cfg(feature = "android")]
use jni::sys::{jbyteArray, jint, jlong};
#[cfg(feature = "android")]
use jni::JNIEnv;

#[cfg(feature = "android")]
use crate::{AudioDecoder, FlacDecoder};

#[cfg(feature = "android")]
#[no_mangle]
pub extern "C" fn Java_app_akroasis_audio_NativeAudioDecoder_createFlacDecoder(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    match FlacDecoder::new() {
        Ok(decoder) => Box::into_raw(Box::new(decoder)) as jlong,
        Err(_) => 0,
    }
}

#[cfg(feature = "android")]
#[no_mangle]
pub extern "C" fn Java_app_akroasis_audio_NativeAudioDecoder_destroyFlacDecoder(
    _env: JNIEnv,
    _class: JClass,
    decoder_ptr: jlong,
) {
    if decoder_ptr != 0 {
        unsafe {
            let _ = Box::from_raw(decoder_ptr as *mut FlacDecoder);
        }
    }
}

#[cfg(feature = "android")]
#[no_mangle]
pub extern "C" fn Java_app_akroasis_audio_NativeAudioDecoder_decodeFlac(
    env: JNIEnv,
    _class: JClass,
    decoder_ptr: jlong,
    data: JByteArray,
) -> jbyteArray {
    if decoder_ptr == 0 {
        return JObject::null().into_raw();
    }

    let decoder = unsafe { &mut *(decoder_ptr as *mut FlacDecoder) };

    let data_bytes = match env.convert_byte_array(data) {
        Ok(bytes) => bytes,
        Err(_) => return JObject::null().into_raw(),
    };

    let decoded = match decoder.decode_file(&data_bytes) {
        Ok(decoded) => decoded,
        Err(_) => return JObject::null().into_raw(),
    };

    let samples_bytes: Vec<u8> = decoded
        .samples
        .iter()
        .flat_map(|&sample| sample.to_le_bytes())
        .collect();

    match env.byte_array_from_slice(&samples_bytes) {
        Ok(array) => array.into_raw(),
        Err(_) => JObject::null().into_raw(),
    }
}

#[cfg(feature = "android")]
#[no_mangle]
pub extern "C" fn Java_app_akroasis_audio_NativeAudioDecoder_getSampleRate(
    _env: JNIEnv,
    _class: JClass,
    decoder_ptr: jlong,
) -> jint {
    if decoder_ptr == 0 {
        return 0;
    }

    let decoder = unsafe { &*(decoder_ptr as *const FlacDecoder) };
    decoder.config().sample_rate as jint
}

#[cfg(feature = "android")]
#[no_mangle]
pub extern "C" fn Java_app_akroasis_audio_NativeAudioDecoder_getChannels(
    _env: JNIEnv,
    _class: JClass,
    decoder_ptr: jlong,
) -> jint {
    if decoder_ptr == 0 {
        return 0;
    }

    let decoder = unsafe { &*(decoder_ptr as *const FlacDecoder) };
    decoder.config().channels as jint
}

#[cfg(feature = "android")]
#[no_mangle]
pub extern "C" fn Java_app_akroasis_audio_NativeAudioDecoder_getBitDepth(
    _env: JNIEnv,
    _class: JClass,
    decoder_ptr: jlong,
) -> jint {
    if decoder_ptr == 0 {
        return 0;
    }

    let decoder = unsafe { &*(decoder_ptr as *const FlacDecoder) };
    decoder.config().bit_depth as jint
}
