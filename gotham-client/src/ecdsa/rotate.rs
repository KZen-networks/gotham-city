// Gotham-city
//
// Copyright 2018 by Kzen Networks (kzencorp.com)
// Gotham city is free software: you can redistribute
// it and/or modify it under the terms of the GNU General Public
// License as published by the Free Software Foundation, either
// version 3 of the License, or (at your option) any later version.
//

use reqwest;
use serde_json;

use super::super::utilities::requests;
use super::super::wallet;
use curv::cryptographic_primitives::twoparty::coin_flip_optimal_rounds;
use kms::ecdsa::two_party::MasterKey2;
use kms::ecdsa::two_party::*;
use kms::rotation::two_party::party2::Rotation2;
use multi_party_ecdsa::protocols::two_party_ecdsa::lindell_2017::*;

const ROT_PATH_PRE: &str = "ecdsa/rotate";

pub fn rotate_master_key(wallet: wallet::Wallet, client: &reqwest::Client) -> wallet::Wallet {
    let id = &wallet.private_shares.id.clone();
    let res_body = requests::post(client, &format!("{}/{}/first", ROT_PATH_PRE, id)).unwrap();

    let coin_flip_party1_first_message: coin_flip_optimal_rounds::Party1FirstMessage =
        serde_json::from_str(&res_body).unwrap();

    let coin_flip_party2_first_message =
        Rotation2::key_rotate_first_message(&coin_flip_party1_first_message);

    let body = &coin_flip_party2_first_message;

    let res_body = requests::postb(
        client,
        &format!("{}/{}/second", ROT_PATH_PRE, id.clone()),
        body,
    )
    .unwrap();

    let (coin_flip_party1_second_message, rotation_party1_first_message): (
        coin_flip_optimal_rounds::Party1SecondMessage,
        party1::RotationParty1Message1,
    ) = serde_json::from_str(&res_body).unwrap();

    let random2 = Rotation2::key_rotate_second_message(
        &coin_flip_party1_second_message,
        &coin_flip_party2_first_message,
        &coin_flip_party1_first_message,
    );

    let result_rotate_party_one_first_message = wallet
        .private_shares
        .master_key
        .rotate_first_message(&random2, &rotation_party1_first_message);
    if result_rotate_party_one_first_message.is_err() {
        panic!("rotation failed");
    }

    let (rotation_party_two_first_message, party_two_pdl_chal, party_two_paillier) =
        result_rotate_party_one_first_message.unwrap();

    let body = &rotation_party_two_first_message;

    let res_body = requests::postb(
        client,
        &format!("{}/{}/third", ROT_PATH_PRE, id.clone()),
        body,
    )
    .unwrap();

    let rotation_party1_second_message: party_one::PDLFirstMessage =
        serde_json::from_str(&res_body).unwrap();

    let rotation_party_two_second_message = MasterKey2::rotate_second_message(&party_two_pdl_chal);

    let body = &rotation_party_two_second_message;

    let res_body = requests::postb(
        client,
        &format!("{}/{}/fourth", ROT_PATH_PRE, id.clone()),
        body,
    )
    .unwrap();

    let rotation_party1_third_message: party_one::PDLSecondMessage =
        serde_json::from_str(&res_body).unwrap();

    let result_rotate_party_one_third_message =
        wallet.private_shares.master_key.rotate_third_message(
            &random2,
            &party_two_paillier,
            &party_two_pdl_chal,
            &rotation_party1_second_message,
            &rotation_party1_third_message,
        );
    if result_rotate_party_one_third_message.is_err() {
        panic!("rotation failed");
    }

    let party_two_master_key_rotated = result_rotate_party_one_third_message.unwrap();

    let private_shares = wallet::PrivateShares {
        id: wallet.private_shares.id.clone(),
        master_key: party_two_master_key_rotated,
    };
    wallet::Wallet {
        id: wallet.id.clone(),
        network: wallet.network.clone(),
        private_shares,
        last_derived_pos: wallet.last_derived_pos.clone(),
        addresses_derivation_map: wallet.addresses_derivation_map,
    }
}
