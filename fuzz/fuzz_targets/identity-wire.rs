#![no_main]

use libfuzzer_sys::fuzz_target;
use serde::{de::DeserializeOwned, Serialize};
use tidex::foundation::digest::Sha256Digest;
use tidex::foundation::identity::{
    AcquisitionId, ArchitectureId, CapabilityId, LearningTargetId, ModelId, ObservationId,
    PortId, ProbeId, SessionId, SkillId, SourceRevision, TensorId,
};

fn assert_wire_round_trip<T>(value: &T)
where
    T: Serialize + DeserializeOwned + PartialEq + core::fmt::Debug,
{
    let encoded = serde_json::to_vec(value).expect("accepted identity must serialize");
    let decoded: T =
        serde_json::from_slice(&encoded).expect("serialized accepted identity must deserialize");
    assert_eq!(&decoded, value);
}

macro_rules! exercise_id {
    ($ty:ty, $input:expr) => {
        if let Ok(value) = <$ty>::parse($input) {
            assert_wire_round_trip(&value);
        }
    };
}

fuzz_target!(|bytes: &[u8]| {
    let Ok(text) = core::str::from_utf8(bytes) else {
        return;
    };

    exercise_id!(SessionId, text);
    exercise_id!(ObservationId, text);
    exercise_id!(ModelId, text);
    exercise_id!(ArchitectureId, text);
    exercise_id!(LearningTargetId, text);
    exercise_id!(ProbeId, text);
    exercise_id!(AcquisitionId, text);
    exercise_id!(PortId, text);
    exercise_id!(SkillId, text);
    exercise_id!(TensorId, text);
    exercise_id!(CapabilityId, text);
    exercise_id!(SourceRevision, text);

    if let Ok(digest) = Sha256Digest::parse(text) {
        assert_eq!(digest.as_str().len(), Sha256Digest::HEX_LEN);
        assert_wire_round_trip(&digest);
        assert_eq!(Sha256Digest::parse(digest.as_str()).unwrap(), digest);
    }
});
