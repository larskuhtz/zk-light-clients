use crate::error::LightClientError;
use wp1_sdk::utils::BabyBearPoseidon2;
use wp1_sdk::{ProverClient, SP1ProofWithIO, SP1Stdin};

#[allow(dead_code)]
fn sig_verification(
    client: &ProverClient,
    ledger_info_w_sig: &[u8],
) -> Result<(SP1ProofWithIO<BabyBearPoseidon2>, bool), LightClientError> {
    use wp1_sdk::utils;
    utils::setup_logger();

    let mut stdin = SP1Stdin::new();

    stdin.write(&ledger_info_w_sig);

    let mut proof = client
        .prove(aptos_programs::bench::SIGNATURE_VERIFICATION_PROGRAM, stdin)
        .map_err(|err| LightClientError::ProvingError {
            program: "signature-verification".to_string(),
            source: err.into(),
        })?;

    // Read output.
    let success = proof.public_values.read::<bool>();

    Ok((proof, success))
}

#[cfg(test)]
#[cfg(feature = "aptos")]
mod test {
    use crate::error::LightClientError;
    use wp1_sdk::{ProverClient, SP1Stdin};

    fn sig_execute(ledger_info_w_sig: &[u8]) -> Result<(), LightClientError> {
        use wp1_sdk::utils;
        utils::setup_logger();

        let mut stdin = SP1Stdin::new();

        stdin.write(&ledger_info_w_sig);

        ProverClient::execute(
            aptos_programs::bench::SIGNATURE_VERIFICATION_PROGRAM,
            &stdin,
        )
        .map_err(|err| LightClientError::ProvingError {
            program: "signature-verification".to_string(),
            source: err.into(),
        })?;

        Ok(())
    }

    #[test]
    fn test_sig_execute() {
        use aptos_lc_core::aptos_test_utils::wrapper::AptosWrapper;
        use aptos_lc_core::NBR_VALIDATORS;
        use std::time::Instant;

        const AVERAGE_SIGNERS_NBR: usize = 95;

        let mut aptos_wrapper = AptosWrapper::new(30000, NBR_VALIDATORS, AVERAGE_SIGNERS_NBR);

        aptos_wrapper.generate_traffic();
        aptos_wrapper.commit_new_epoch();

        let ledger_info_with_signature = aptos_wrapper.get_latest_li_bytes().unwrap();

        println!("Starting execution of signature verification...");
        let start = Instant::now();
        sig_execute(&ledger_info_with_signature).unwrap();
        println!("Execution took {:?}", start.elapsed());
    }

    #[test]
    fn test_sig_prove() {
        use super::*;
        use aptos_lc_core::aptos_test_utils::wrapper::AptosWrapper;
        use aptos_lc_core::NBR_VALIDATORS;
        use std::time::Instant;
        use wp1_sdk::ProverClient;

        const AVERAGE_SIGNERS_NBR: usize = 95;

        let mut aptos_wrapper = AptosWrapper::new(30000, NBR_VALIDATORS, AVERAGE_SIGNERS_NBR);

        aptos_wrapper.generate_traffic();
        aptos_wrapper.commit_new_epoch();

        let ledger_info_with_signature = aptos_wrapper.get_latest_li_bytes().unwrap();

        let client = ProverClient::new();

        let start = Instant::now();
        println!("Starting generation of signature verification proof...");
        let (proof, _) = sig_verification(&client, &ledger_info_with_signature).unwrap();
        println!("Proving took {:?}", start.elapsed());

        let start = Instant::now();
        println!("Starting verification of signature verification proof...");
        client
            .verify(
                aptos_programs::bench::SIGNATURE_VERIFICATION_PROGRAM,
                &proof,
            )
            .unwrap();
        println!("Verification took {:?}", start.elapsed());
    }
}