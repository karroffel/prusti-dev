// © 2019, ETH Zurich
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! A module that invokes the verifier `prusti-viper`

use prusti_interface::data::VerificationResult;
use prusti_interface::data::VerificationTask;
use prusti_interface::environment::Environment;
use prusti_interface::report::user;
use prusti_interface::specifications::TypedSpecificationMap;
use prusti_interface::verifier::VerificationContext;
use prusti_interface::verifier::Verifier;
use prusti_interface::verifier::VerifierBuilder;
use prusti_viper::verifier::VerifierBuilder as ViperVerifierBuilder;
use rustc_driver::driver;
use std::time::Instant;

/// Verify a (typed) specification on compiler state.
pub fn verify<'r, 'a: 'r, 'tcx: 'a>(
    state: &'r mut driver::CompileState<'a, 'tcx>,
    spec: TypedSpecificationMap,
) {
    trace!("[verify] enter");

    let env = Environment::new(state);

    if env.has_errors() {
        warn!("The compiler reported an error, so the program will not be verified.");
    } else {
        debug!("Specification consists of {} elements.", spec.len());

        debug!("Prepare verification task...");
        let annotated_procedures = env.get_annotated_procedures();
        let verification_task = VerificationTask {
            procedures: annotated_procedures,
        };
        debug!("Verification task: {:?}", &verification_task);

        user::message(format!(
            "Verification of {} items...",
            verification_task.procedures.len()
        ));

        let verification_result = if verification_task.procedures.is_empty() {
            VerificationResult::Success
        } else {
            debug!("Dump borrow checker info...");
            env.dump_borrowck_info(&verification_task.procedures);

            debug!("Prepare verifier...");
            let jvm_start = Instant::now();
            let verifier_builder = ViperVerifierBuilder::new();
            let verification_context = VerifierBuilder::new_verification_context(&verifier_builder);
            let jvm_duration = jvm_start.elapsed();
            info!(
                "JVM startup ({}.{} seconds)",
                jvm_duration.as_secs(),
                jvm_duration.subsec_millis() / 10
            );

            let verifier_start = Instant::now();
            let mut verifier = verification_context.new_verifier(&env, &spec);
            let verifier_duration = verifier_start.elapsed();
            info!(
                "Verifier startup ({}.{} seconds)",
                verifier_duration.as_secs(),
                verifier_duration.subsec_millis() / 10
            );

            debug!("Run verifier...");
            let verification_result = verifier.verify(&verification_task);
            debug!("Verifier returned {:?}", verification_result);

            verification_result
        };

        match verification_result {
            VerificationResult::Success => {
                user::message(format!(
                    "Successful verification of {} items",
                    verification_task.procedures.len()
                ));
            }
            VerificationResult::Failure => {
                user::message("Verification failed");
                assert!(env.has_errors());
            }
        };
    }

    trace!("[verify] exit");
}
