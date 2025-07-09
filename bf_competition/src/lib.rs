// Organization: Rockin' Freeworld Foundation
// Project: BRAINFROG LLC
// Author: RenegadeJpg
// Title: BF Art Competition Contract
// Description: A Soroban smart contract for managing art competitions, allowing artists to submit artworks, vote on them, and determine winners.
// This contract includes features for artist registration, competition management, voting, and artist metadata storage.
// Version: 1.0.1
#![no_std]

use core::convert::TryInto;
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, vec, Address, BytesN, Env, IntoVal, Map, String,
    Symbol, Vec,
};

#[derive(Clone)]
#[contracttype]
pub struct ArtworkMetadata {
    pub artwork_name: String,
    pub description: String,
    pub img_url: String,
}

#[derive(Clone)]
#[contracttype]
pub struct VotingEligibility {
    pub can_vote: bool,
    pub has_voted: bool,
    pub current_balance: u64,
    pub min_required: u64,
    pub voting_active: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct Competition {
    pub id: String,
    pub id_description: String,
    pub token: Address,
    pub artist_add_start: u64,
    pub artist_add_end: u64,
    pub vote_start: u64,
    pub vote_end: u64,
    pub min_vote_tokens: u64,
    pub artists: Vec<(Address, String)>,
    pub votes: Map<String, u64>,
    pub vote_log: Map<Address, String>,
    pub finalized: bool,
    pub winner: Option<String>,
    pub pot: u64,
    pub artist_metadata: Map<String, Vec<ArtworkMetadata>>,
    pub share_ratio: Vec<u32>,
}

#[derive(Clone)]
#[contracttype]
pub struct CompetitionStatus {
    pub id: String,
    pub competition: Competition,
    pub is_submission_active: bool,
    pub is_voting_active: bool,
    pub is_finalized: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct ArtistRanking {
    pub artist: String,
    pub votes: u64,
    pub rank: u32,
    pub is_winner: bool,
}

#[derive(Clone)]
#[contracttype]
pub struct VoteHistory {
    pub voter: Address,
    pub artist: String,
    pub timestamp: u64,
}

#[derive(Clone)]
#[contracttype]
pub struct Medium {
    pub name: String,
    pub description: String,
}

#[derive(Clone)]
#[contracttype]
pub struct Network {
    pub name: String,
    pub chain_id: String,
}

#[derive(Clone)]
#[contracttype]
pub struct ArtistInfo {
    pub registered: bool,
    pub name: String,
    pub bio: String,
    pub img_url: String,
    pub website: String,
    pub mediums: Vec<Medium>,
    pub blockchains: Vec<Network>,
    pub competitions_participated: u32,
    pub competitions_won: u32,
}

#[contract]
pub struct CompetitionContract;

#[contractimpl]
impl CompetitionContract {
    #[inline(always)]
    ///*** Functions for Admins ***
    ///
    /// Initialize the contract with two admins
    pub fn __constructor(env: Env, admin1: Address, admin2: Address) {
        env.storage()
            .instance()
            .set(&Symbol::new(&env, "admin1"), &admin1);
        env.storage()
            .instance()
            .set(&Symbol::new(&env, "admin2"), &admin2);
    }
    /// Update one or both admins of the contract {Only admins can update}
    pub fn update_admins(
        env: Env,
        from: Address,
        new_admin1: Option<Address>,
        new_admin2: Option<Address>,
    ) {
        // Check if caller is one of the current admins
        let admin1: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "admin1"))
            .unwrap();
        let admin2: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "admin2"))
            .unwrap();
        assert!(
            from == admin1 || from == admin2,
            "Only admins can update admins"
        );

        from.require_auth();

        // Update admin1 if provided, otherwise keep the current value
        if let Some(addr) = new_admin1 {
            env.storage()
                .instance()
                .set(&Symbol::new(&env, "admin1"), &addr);
        }

        // Update admin2 if provided, otherwise keep the current value
        if let Some(addr) = new_admin2 {
            env.storage()
                .instance()
                .set(&Symbol::new(&env, "admin2"), &addr);
        }
    }
    ///
    /// Manually migrate a single artist from another contract {Only admins can migrate}
    pub fn migrate_single_artist(env: Env, from: Address, artist_address: Address, artist_info: ArtistInfo) {
        // Check if from is one of the two admins
        let admin1: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "admin1"))
            .unwrap();
        let admin2: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "admin2"))
            .unwrap();
        assert!(
            from == admin1 || from == admin2,
            "Only admins can migrate artist info"
        );

        from.require_auth();

        let artist_info_key = Symbol::new(&env, "artist_info");
        let mut all_info: Map<Address, ArtistInfo> = env
            .storage()
            .instance()
            .get(&artist_info_key)
            .unwrap_or(Map::new(&env));

        // Add the artist
        all_info.set(artist_address, artist_info);
        env.storage().instance().set(&artist_info_key, &all_info);
    }
    
    ///
    /// Create a new competition {Only admins can create}
    pub fn create_competition(
        env: Env,
        from: Address,
        id: String,
        id_description: String,
        artist_add_start: u64,
        artist_add_end: u64,
        vote_start: u64,
        vote_end: u64,
        token: Address,
        min_vote_tokens: u64,
    ) {
        assert!(
            artist_add_start < artist_add_end,
            "Invalid submission window"
        );
        assert!(
            vote_start > artist_add_end,
            "Vote start must be after submission end"
        );
        assert!(vote_end > vote_start, "Invalid voting window");

        // Check if competition ID already exists
        assert!(
            env.storage()
                .instance()
                .get::<String, Competition>(&id)
                .is_none(),
            "Competition ID already exists"
        );

        // Check if from is one of the two admins
        let admin1_opt = env
            .storage()
            .instance()
            .get::<Symbol, Address>(&Symbol::new(&env, "admin1"));
        let admin2_opt = env
            .storage()
            .instance()
            .get::<Symbol, Address>(&Symbol::new(&env, "admin2"));

        assert!(
            admin1_opt.is_some() && admin2_opt.is_some(),
            "Contract not initialized: missing admins"
        );

        let admin1 = admin1_opt.unwrap();
        let admin2 = admin2_opt.unwrap();
        assert!(from == admin1 || from == admin2, "Only admins can create");

        from.require_auth();

        let comp = Competition {
            id: id.clone(),
            id_description,
            artist_add_start,
            artist_add_end,
            vote_start,
            vote_end,
            token,
            min_vote_tokens,
            artists: Vec::new(&env),
            votes: Map::new(&env),
            vote_log: Map::new(&env),
            finalized: false,
            winner: None,
            pot: 0,
            artist_metadata: Map::new(&env),
            share_ratio: Vec::from_array(&env, [50u32, 30u32, 20u32]), // Default share ratio
        };

        env.storage().instance().set(&id, &comp);

        // Add competition ID to the list
        let competition_list_key = Symbol::new(&env, "comp_list");
        let mut competition_ids: Vec<String> = env
            .storage()
            .instance()
            .get(&competition_list_key)
            .unwrap_or(Vec::new(&env));
        competition_ids.push_back(id.clone());
        env.storage()
            .instance()
            .set(&competition_list_key, &competition_ids);
    }

    /// Delete a competition and emergency withdraw funds {Only admin can delete}
    pub fn delete_competition(env: Env, id: String, from: Address) {
        let comp: Competition = env.storage().instance().get(&id).unwrap();

        // Check if from is one of the two admins
        let admin1: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "admin1"))
            .unwrap();
        let admin2: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "admin2"))
            .unwrap();
        assert!(from == admin1 || from == admin2, "Only admins can delete");

        from.require_auth();

        // Remove competition from storage before external calls to avoid reentrancy
        env.storage().instance().remove(&id);

        // Emergency withdraw any remaining funds to the sender (from)
        if comp.pot > 0 {
            let decimals: u32 =
                env.invoke_contract(&comp.token, &Symbol::new(&env, "decimals"), vec![&env]);
            let multiplier = 10u64.checked_pow(decimals).expect("Overflow in decimals");
            let amount_stroop = comp
                .pot
                .checked_mul(multiplier)
                .expect("Overflow in pot transfer");

            env.invoke_contract::<()>(
                &comp.token,
                &symbol_short!("transfer"),
                vec![
                    &env,
                    env.current_contract_address().into_val(&env),
                    from.into_val(&env),
                    (amount_stroop as i128).into_val(&env),
                ],
            );
        }

        // Remove from competition list
        let competition_list_key = Symbol::new(&env, "comp_list");
        let competition_ids: Vec<String> = env
            .storage()
            .instance()
            .get(&competition_list_key)
            .unwrap_or(Vec::new(&env));

        let mut new_ids = Vec::new(&env);
        for comp_id in competition_ids.iter() {
            if comp_id != id {
                new_ids.push_back(comp_id);
            }
        }
        env.storage()
            .instance()
            .set(&competition_list_key, &new_ids);

        // Clean up vote history for this competition
        let vote_history_key = Symbol::new(&env, "vote_hist");
        let mut all_vote_history: Map<String, Vec<VoteHistory>> = env
            .storage()
            .instance()
            .get(&vote_history_key)
            .unwrap_or(Map::new(&env));

        all_vote_history.remove(id);
        env.storage()
            .instance()
            .set(&vote_history_key, &all_vote_history);
    }

    ///
    /// Remove an artist from a competition {Only admin can remove}
    pub fn remove_artist(env: Env, id: String, from: Address, artist_name: String) {
        let mut comp: Competition = env.storage().instance().get(&id).unwrap();

        // Check if from is one of the two admins
        let admin1: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "admin1"))
            .unwrap();
        let admin2: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "admin2"))
            .unwrap();
        assert!(
            from == admin1 || from == admin2,
            "Only admins can remove artists"
        );

        from.require_auth();

        // Find and remove the artist from the artists list
        let mut new_artists = Vec::new(&env);
        let mut artist_found = false;
        let mut artist_address_to_remove: Option<Address> = None;
        for (addr, name) in comp.artists.iter() {
            if name != artist_name {
                new_artists.push_back((addr, name));
            } else {
                artist_found = true;
                artist_address_to_remove = Some(addr);
            }
        }

        assert!(artist_found, "Artist not found in competition");
        comp.artists = new_artists;

        // Remove artist metadata
        comp.artist_metadata.remove(artist_name.clone());

        // Remove any votes for this artist
        comp.votes.remove(artist_name.clone());

        // Remove any vote logs that voted for this artist and update vote history
        let vote_history_key = Symbol::new(&env, "vote_hist");
        let mut all_vote_history: Map<String, Vec<VoteHistory>> = env
            .storage()
            .instance()
            .get(&vote_history_key)
            .unwrap_or(Map::new(&env));

        // Filter out votes for the removed artist from vote history
        let comp_history = all_vote_history.get(id.clone()).unwrap_or(Vec::new(&env));
        let mut new_history = Vec::new(&env);
        let mut voters_to_clear = Vec::new(&env);

        for vote_record in comp_history.iter() {
            if vote_record.artist != artist_name {
                new_history.push_back(vote_record);
            } else {
                voters_to_clear.push_back(vote_record.voter.clone());
            }
        }

        // Clear vote log entries for voters who voted for the removed artist
        for voter in voters_to_clear.iter() {
            comp.vote_log.remove(voter.clone());
        }

        all_vote_history.set(id.clone(), new_history);
        env.storage()
            .instance()
            .set(&vote_history_key, &all_vote_history);

        env.storage().instance().set(&id, &comp);

        // Remove artist info from mapping if present
        if let Some(addr) = artist_address_to_remove {
            let artist_info_key = Symbol::new(&env, "artist_info");
            let mut all_info: Map<Address, ArtistInfo> = env
                .storage()
                .instance()
                .get(&artist_info_key)
                .unwrap_or(Map::new(&env));
            all_info.remove(addr);
            env.storage().instance().set(&artist_info_key, &all_info);
        }
    }
    ///
    /// Remove a registered artist from the global artist registry {Only admin can remove}
    pub fn remove_registered_artist(env: Env, from: Address, artist_address: Address) {
        // Check if from is one of the two admins
        let admin1: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "admin1"))
            .unwrap();
        let admin2: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "admin2"))
            .unwrap();
        assert!(
            from == admin1 || from == admin2,
            "Only admins can remove registered artists"
        );

        from.require_auth();

        let artist_info_key = Symbol::new(&env, "artist_info");
        let mut all_info: Map<Address, ArtistInfo> = env
            .storage()
            .instance()
            .get(&artist_info_key)
            .unwrap_or(Map::new(&env));

        // Check if artist exists
        assert!(
            all_info.contains_key(artist_address.clone()),
            "Artist not found in registry"
        );

        // Remove the artist from the registry
        all_info.remove(artist_address);
        env.storage().instance().set(&artist_info_key, &all_info);
    }

    /// Upgrade the contract to a new implementation {Only admins can upgrade}
    pub fn upgrade(env: Env, from: Address, new_wasm_hash: BytesN<32>) {
        // Check if from is one of the two admins
        let admin1: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "admin1"))
            .unwrap();
        let admin2: Address = env
            .storage()
            .instance()
            .get(&Symbol::new(&env, "admin2"))
            .unwrap();
        assert!(
            from == admin1 || from == admin2,
            "Only admins can upgrade contract"
        );

        from.require_auth();

        // Upgrade the contract to the new WASM hash
        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    /// ***{User Write Fn for Art Competition}***
    ///
    /// Vote for an artist in a competition (one vote per wallet)
    pub fn vote(env: Env, id: String, from: Address, artist: String) {
        let mut comp: Competition = env.storage().instance().get(&id).unwrap();

        // Authenticate first before any external calls
        from.require_auth();

        // Use the voting eligibility check
        let eligibility = Self::check_voting_eligibility(env.clone(), id.clone(), from.clone());
        assert!(eligibility.can_vote, "Not eligible to vote");
        assert!(!eligibility.has_voted, "Already voted");

        // Verify artist exists
        assert!(
            comp.artists.iter().any(|(_, name)| name == artist),
            "Invalid artist"
        );

        // Update vote_log before any external calls
        comp.vote_log.set(from.clone(), artist.clone());

        // Record the vote
        let current_votes = comp.votes.get(artist.clone()).unwrap_or(0);
        comp.votes.set(artist.clone(), current_votes + 1);

        // Store vote history using the competition's vote log directly
        // We'll store vote history within the competition structure by extending it
        let vote_record = VoteHistory {
            voter: from,
            artist,
            timestamp: env.ledger().timestamp(),
        };

        // Get existing vote history for this competition
        let vote_history_key = Symbol::new(&env, "vote_hist");
        let mut all_vote_history: Map<String, Vec<VoteHistory>> = env
            .storage()
            .instance()
            .get(&vote_history_key)
            .unwrap_or(Map::new(&env));

        let mut comp_history = all_vote_history.get(id.clone()).unwrap_or(Vec::new(&env));
        comp_history.push_back(vote_record);
        all_vote_history.set(id.clone(), comp_history);

        env.storage()
            .instance()
            .set(&vote_history_key, &all_vote_history);
        env.storage().instance().set(&id, &comp);
    }
    ///
    /// Fund the competition pot
    pub fn fund_pot(env: Env, id: String, from: Address, amount: u64) {
        let mut comp: Competition = env.storage().instance().get(&id).unwrap();

        from.require_auth();

        // Get token decimals
        let decimals: u32 =
            env.invoke_contract(&comp.token, &Symbol::new(&env, "decimals"), vec![&env]);
        let multiplier = 10u64.checked_pow(decimals).expect("Overflow in decimals");
        let amount_stroop = amount
            .checked_mul(multiplier)
            .expect("Overflow in fund_pot");

        // Update pot before external call
        comp.pot = comp.pot.checked_add(amount).expect("Overflow in pot");
        env.storage().instance().set(&id, &comp);

        // Transfer tokens from sender to contract
        env.invoke_contract::<()>(
            &comp.token,
            &symbol_short!("transfer"),
            vec![
                &env,
                from.into_val(&env),
                env.current_contract_address().into_val(&env),
                (amount_stroop as i128).into_val(&env),
            ],
        );
    }
    ///
    /// Pays the competition winners
    pub fn pay_winners(env: Env, id: String) {
        let mut comp: Competition = env.storage().instance().get(&id).unwrap();

        let now = env.ledger().timestamp();
        assert!(now > comp.vote_end, "Voting still active");

        // If not finalized yet, determine the winner first
        if !comp.finalized {
            Self::internal_finalize(&env, &mut comp);
            env.storage().instance().set(&id, &comp);
        }

        // Always attempt prize distribution if there's a pot and winner
        // This allows to distribute prizes even after auto-finalization
        if comp.pot == 0 || comp.winner.is_none() {
            return;
        }

        let decimals: u32 = match env.try_invoke_contract::<u32, soroban_sdk::InvokeError>(
            &comp.token,
            &Symbol::new(&env, "decimals"),
            vec![&env],
        ) {
            Ok(Ok(d)) => d,
            Ok(Err(_)) | Err(_) => {
                // Winner already determined, just return
                return;
            }
        };

        let multiplier = 10u64.pow(decimals);

        // Collect all artists with their votes and sort them for prize distribution
        let mut artist_votes = Vec::new(&env);
        for (_, artist_name) in comp.artists.iter() {
            let votes = comp.votes.get(artist_name.clone()).unwrap_or(0);
            artist_votes.push_back((artist_name.clone(), votes));
        }

        // Sort by votes (descending) - manual bubble sort
        let len = artist_votes.len();
        for i in 0..len {
            for j in 0..(len - 1 - i) {
                let current_votes = artist_votes.get(j).unwrap().1;
                let next_votes = artist_votes.get(j + 1).unwrap().1;
                if current_votes < next_votes {
                    let temp = artist_votes.get(j).unwrap();
                    let next = artist_votes.get(j + 1).unwrap();
                    artist_votes.set(j, next);
                    artist_votes.set(j + 1, temp);
                }
            }
        }

        // Group artists by vote count
        let mut grouped = Vec::new(&env);
        let mut current_votes = None;
        let mut current_group = Vec::new(&env);

        for (i, (artist_symbol, votes)) in artist_votes.iter().enumerate() {
            if i == 0 {
                current_votes = Some(votes);
                current_group.push_back(artist_symbol);
            } else {
                let prev_votes = current_votes.unwrap();
                if votes == prev_votes {
                    current_group.push_back(artist_symbol);
                } else {
                    grouped.push_back((prev_votes, current_group.clone()));
                    current_group = Vec::new(&env);
                    current_group.push_back(artist_symbol);
                    current_votes = Some(votes);
                }
            }
        }
        if let Some(count) = current_votes {
            grouped.push_back((count, current_group));
        }

        let mut rank_index = 0usize;
        let share = comp.share_ratio.clone();
        let mut total_paid = 0u64;

        for group_data in grouped.iter() {
            let (votes_in_group, group_artists) = group_data;
            if votes_in_group == 0 {
                break;
            }

            let group_size = group_artists.len();
            let mut combined_share = 0u64;
            for offset in 0..group_size {
                let index = rank_index + offset as usize;
                if index < share.len().try_into().unwrap() {
                    combined_share += share.get(index as u32).unwrap() as u64;
                }
            }
            if combined_share == 0 {
                break;
            }

            let share_per_artist = combined_share / (group_size as u64);
            for artist_sym in group_artists.iter() {
                for (addr, name) in comp.artists.iter() {
                    if name == artist_sym {
                        let amt_lumen = comp.pot * share_per_artist / 100;
                        let amt_stroop = amt_lumen * multiplier;
                        if amt_stroop > 0 {
                            match env.try_invoke_contract::<(), soroban_sdk::InvokeError>(
                                &comp.token,
                                &symbol_short!("transfer"),
                                vec![
                                    &env,
                                    env.current_contract_address().into_val(&env),
                                    addr.into_val(&env),
                                    (amt_stroop as i128).into_val(&env),
                                ],
                            ) {
                                Ok(Ok(())) => {
                                    total_paid += amt_lumen;
                                }
                                Ok(Err(_)) | Err(_) => {
                                    // Transfer failed, continue with next artist
                                }
                            }
                        }
                        break;
                    }
                }
            }
            rank_index += group_size as usize;
            if rank_index >= share.len().try_into().unwrap() {
                break;
            }
        }

        // Handle leftover distribution proportionally among actual winners
        let leftover = comp.pot - total_paid;
        if leftover > 0 && artist_votes.len() > 0 {
            // Calculate total share percentage used by actual winners
            let mut total_winner_share = 0u64;
            let winners_count = artist_votes
                .iter()
                .take_while(|(_, votes)| *votes > 0)
                .count()
                .min(share.len().try_into().unwrap());

            for i in 0..winners_count {
                if i < share.len().try_into().unwrap() {
                    total_winner_share += share.get(i as u32).unwrap() as u64;
                }
            }

            if total_winner_share > 0 {
                // Distribute leftover proportionally based on their original share
                for i in 0..winners_count {
                    if i < artist_votes.len().try_into().unwrap()
                        && artist_votes.get(i.try_into().unwrap()).unwrap().1 > 0
                        && i < share.len().try_into().unwrap()
                    {
                        let artist_name =
                            artist_votes.get(i.try_into().unwrap()).unwrap().0.clone();
                        let artist_share = share.get(i as u32).unwrap() as u64;

                        // Calculate proportional share of leftover
                        let proportional_amount = (leftover * artist_share) / total_winner_share;

                        if proportional_amount > 0 {
                            let mut artist_address: Option<Address> = None;
                            for (addr, name) in comp.artists.iter() {
                                if name == artist_name {
                                    artist_address = Some(addr);
                                    break;
                                }
                            }
                            if let Some(addr) = artist_address {
                                let amt_stroop = proportional_amount * multiplier;
                                if amt_stroop > 0 {
                                    let _ = env
                                        .try_invoke_contract::<(), soroban_sdk::InvokeError>(
                                            &comp.token,
                                            &symbol_short!("transfer"),
                                            vec![
                                                &env,
                                                env.current_contract_address().into_val(&env),
                                                addr.into_val(&env),
                                                (amt_stroop as i128).into_val(&env),
                                            ],
                                        );
                            }
                            }
                        }
                    }
                }
            }
        }

        // Update pot to reflect distributed prizes
        comp.pot = 0; // Set to 0, prevent double distribution
        env.storage().instance().set(&id, &comp);
    }
    ///***{Artist Write Fn for Art Competition}***
    ///
    /// Submit an artist to the competition
    pub fn submit_art(
        env: Env,
        id: String,
        artist_address: Address,
        artist_name: String,
        artwork_name: String,
        description: String,
        img_url: String,
    ) {
        artist_address.require_auth();

        let now = env.ledger().timestamp();

        // Check if competition exists
        let mut comp: Competition = match env.storage().instance().get(&id) {
            Some(competition) => competition,
            None => panic!("Competition not found"),
        };

        // Check submission window
        if now < comp.artist_add_start || now > comp.artist_add_end {
            panic!("Submission window closed");
        }

        // Artists should ensure their symbol names are unique regardless of case
        // Check for duplicate artist names (exact match)
        for (_, name) in comp.artists.iter() {
            if name == artist_name {
                assert!(false, "Artist name already submitted");
            }
        }

        // Check for duplicate metadata
        let mut artworks = comp
            .artist_metadata
            .get(artist_name.clone())
            .unwrap_or_else(|| Vec::new(&env));

        // Check for duplicates in the existing artworks
        for item in artworks.iter() {
            if item.artwork_name == artwork_name {
                panic!("Artwork name already exists");
            }
            if item.description == description {
                panic!("Description already exists");
            }
            if item.img_url == img_url {
                panic!("Image URL already exists");
            }
        }

        artworks.push_back(ArtworkMetadata {
            artwork_name,
            description,
            img_url,
        });

        comp.artist_metadata.set(artist_name.clone(), artworks);

        // Add artist to the list
        comp.artists.push_back((artist_address, artist_name));

        // Save updated competition
        env.storage().instance().set(&id, &comp);
    }

    /// Update artist metadata with optional parameters
    pub fn update_submission_metadata(
        env: Env,
        id: String,
        artist_name: String,
        from: Address,
        artwork_name: Option<String>,
        description: Option<String>,
    ) {
        let mut comp: Competition = env.storage().instance().get(&id).unwrap();

        // Verify the caller is the artist who submitted
        let mut is_artist = false;
        for (addr, name) in comp.artists.iter() {
            if name == artist_name && addr == from {
                is_artist = true;
                break;
            }
        }
        assert!(is_artist, "Only the submitting artist can update metadata");

        from.require_auth();

        // Get current metadata
        let mut artworks = comp.artist_metadata.get(artist_name.clone()).unwrap();

        if artworks.len() > 0 {
            let first_art = artworks.get(0).unwrap();
            let mut updated_art = ArtworkMetadata {
                artwork_name: first_art.artwork_name,
                description: first_art.description,
                img_url: first_art.img_url,
            };
            if let Some(name) = artwork_name {
                updated_art.artwork_name = name;
            }
            if let Some(desc) = description {
                updated_art.description = desc;
            }
            artworks.set(0, updated_art);
        }

        comp.artist_metadata.set(artist_name, artworks);
        env.storage().instance().set(&id, &comp);
    }

    ///***{Artist Write Fn for Artist Registration}***
    ///
    /// Add artist profile info (one per address, unique name)
    pub fn add_artist_info(
        env: Env,
        from: Address,
        name: String,
        bio: String,
        img_url: String,
        website: String,
        mediums: Vec<Medium>,
        blockchains: Vec<Network>,
    ) {
        from.require_auth();

        let artist_info_key = Symbol::new(&env, "artist_info");

        // Load all artist info map: Address -> ArtistInfo
        let mut all_info: Map<Address, ArtistInfo> = env
            .storage()
            .instance()
            .get(&artist_info_key)
            .unwrap_or(Map::new(&env));

        // Check for exact name matches instead of case-insensitive
        for (_, info) in all_info.iter() {
            if info.name == name {
                assert!(false, "Artist name already exists");
            }
        }

        // Save artist info for this address
        let info = ArtistInfo {
            registered: true,
            name,
            bio,
            img_url,
            website,
            mediums,
            blockchains,
            competitions_participated: 0,
            competitions_won: 0,
        };
        all_info.set(from.clone(), info);

        env.storage().instance().set(&artist_info_key, &all_info);
    }

    /// Update artist profile info (only by the artist, optional fields)
    pub fn update_artist_info(
        env: Env,
        from: Address,
        name: Option<String>,
        bio: Option<String>,
        img_url: Option<String>,
        website: Option<String>,
        mediums: Option<Vec<Medium>>,
        blockchains: Option<Vec<Network>>,
    ) {
        from.require_auth();

        let artist_info_key = Symbol::new(&env, "artist_info");
        let mut all_info: Map<Address, ArtistInfo> = env
            .storage()
            .instance()
            .get(&artist_info_key)
            .unwrap_or(Map::new(&env));

        // Must exist to update
        let mut info = all_info.get(from.clone()).expect("Artist info not found");

        // If updating name, check for duplicates (exact match)
        if let Some(ref new_name) = name {
            for (addr, other_info) in all_info.iter() {
                if addr != from && other_info.name == *new_name {
                    assert!(false, "Artist name already exists");
                }
            }
            info.name = new_name.clone();
        }
        if let Some(new_bio) = bio {
            info.bio = new_bio;
        }
        if let Some(new_img) = img_url {
            info.img_url = new_img;
        }
        if let Some(new_website) = website {
            info.website = new_website;
        }
        if let Some(new_mediums) = mediums {
            info.mediums = new_mediums;
        }
        if let Some(new_blockchains) = blockchains {
            info.blockchains = new_blockchains;
        }

        // registered, competitions_participated, competitions_won remain unchanged
        all_info.set(from.clone(), info);
        env.storage().instance().set(&artist_info_key, &all_info);
    }

    /// ***{Read Functions}***
    ///
    /// Get all active competitions based on current timestamp
    pub fn get_active_competitions(env: Env) -> Vec<CompetitionStatus> {
        let now = env.ledger().timestamp();
        let mut active_competitions = Vec::new(&env);
        let competition_list_key = Symbol::new(&env, "comp_list");
        let competition_ids: Vec<String> = env
            .storage()
            .instance()
            .get(&competition_list_key)
            .unwrap_or(Vec::new(&env));

        for id in competition_ids.iter() {
            if let Some(mut comp) = env.storage().instance().get::<String, Competition>(&id) {
                // Automatically finalize if time is past voting and not already finalized
                if !comp.finalized && now > comp.vote_end {
                    Self::internal_finalize(&env, &mut comp);
                    // Save the updated competition after internal finalization
                    env.storage().instance().set(&id, &comp);
                }

                let is_submission_active =
                    now >= comp.artist_add_start && now <= comp.artist_add_end;
                let is_voting_active = now >= comp.vote_start && now <= comp.vote_end;
                let is_finalized = comp.finalized;

                if is_submission_active || is_voting_active || (now <= comp.vote_end + 86400) {
                    active_competitions.push_back(CompetitionStatus {
                        id: id.clone(),
                        competition: comp,
                        is_submission_active,
                        is_voting_active,
                        is_finalized,
                    });
                }
            }
        }

        active_competitions
    }

    /// Get the competition details
    pub fn get_competition(env: Env, id: String) -> Competition {
        env.storage().instance().get(&id).unwrap()
    }

    /// View all submitted artists and their metadata for a competition
    pub fn get_comp_artists(env: Env, id: String) -> Vec<(String, ArtworkMetadata)> {
        let comp: Competition = env.storage().instance().get(&id).unwrap();

        let mut result = Vec::new(&env);
        for (artist, artworks) in comp.artist_metadata.iter() {
            for art in artworks.iter() {
                result.push_back((artist.clone(), art));
            }
        }

        result
    }

    /// Get the total pot for a competition
    pub fn get_pot(env: Env, id: String) -> u64 {
        let comp: Competition = env.storage().instance().get(&id).unwrap();
        comp.pot
    }

    /// Get the minimum token amount required to vote in a competition
    pub fn get_min_vote_tokens(env: Env, id: String) -> u64 {
        let comp: Competition = env.storage().instance().get(&id).unwrap();
        comp.min_vote_tokens
    }

    /// Get all contestants ranked by votes from winner to last place
    pub fn get_winner(env: Env, id: String) -> Vec<ArtistRanking> {
        let comp: Competition = env.storage().instance().get(&id).unwrap();

        // Collect all artists with their votes
        let mut artist_votes = Vec::new(&env);
        for artist in comp.artists.iter() {
            let votes = comp.votes.get(artist.1.clone()).unwrap_or(0);
            artist_votes.push_back((artist.1.clone(), votes));
        }

        // Sort by votes (descending)
        let len = artist_votes.len();
        for i in 0..len {
            for j in 0..(len - 1 - i) {
                let current_votes = artist_votes.get(j).unwrap().1;
                let next_votes = artist_votes.get(j + 1).unwrap().1;
                if current_votes < next_votes {
                    let temp = artist_votes.get(j).unwrap();
                    let next = artist_votes.get(j + 1).unwrap();
                    artist_votes.set(j, next);
                    artist_votes.set(j + 1, temp);
                }
            }
        }

        // Create ranking with positions
        let mut rankings = Vec::new(&env);
        for (i, (artist, votes)) in artist_votes.iter().enumerate() {
            rankings.push_back(ArtistRanking {
                artist,
                votes,
                rank: (i + 1) as u32,
                is_winner: i == 0 && votes > 0,
            });
        }

        rankings
    }

    /// Get all artists and their info stored on the contract
    pub fn get_artists(env: Env) -> Vec<(Address, ArtistInfo)> {
        let artist_info_key = Symbol::new(&env, "artist_info");
        let all_info: Map<Address, ArtistInfo> = env
            .storage()
            .instance()
            .get(&artist_info_key)
            .unwrap_or(Map::new(&env));
        let mut result = Vec::new(&env);
        for (addr, info) in all_info.iter() {
            result.push_back((addr, info));
        }
        result
    }

    /// Get artist info for a specific address
    pub fn get_artist_info(env: Env, address: Address) -> Option<ArtistInfo> {
        let artist_info_key = Symbol::new(&env, "artist_info");
        let all_info: Map<Address, ArtistInfo> = env
            .storage()
            .instance()
            .get(&artist_info_key)
            .unwrap_or(Map::new(&env));
        all_info.get(address)
    }

    /// Get vote history for a competition
    pub fn get_vote_history(env: Env, id: String) -> Vec<VoteHistory> {
        let vote_history_key = Symbol::new(&env, "vote_hist");
        let all_vote_history: Map<String, Vec<VoteHistory>> = env
            .storage()
            .instance()
            .get(&vote_history_key)
            .unwrap_or(Map::new(&env));

        all_vote_history.get(id).unwrap_or(Vec::new(&env))
    }

    /// check if an wallet has registered artist info
    pub fn has_registered(env: Env, address: Address) -> bool {
        let artist_info_key = Symbol::new(&env, "artist_info");
        let all_info: Map<Address, ArtistInfo> = env
            .storage()
            .instance()
            .get(&artist_info_key)
            .unwrap_or(Map::new(&env));
        all_info.contains_key(address)
    }

    /// Check if a voter voted
    pub fn has_voted(env: Env, id: String, voter: Address) -> Option<String> {
        let comp: Competition = env.storage().instance().get(&id).unwrap();
        comp.vote_log.get(voter)
    }

    /// Check if a user can vote
    pub fn check_voting_eligibility(env: Env, id: String, voter: Address) -> VotingEligibility {
        let comp: Competition = env.storage().instance().get(&id).unwrap();
        let now = env.ledger().timestamp();

        let voting_active = now >= comp.vote_start && now <= comp.vote_end;

        // Get token decimals
        let decimals: u32 =
            env.invoke_contract(&comp.token, &Symbol::new(&env, "decimals"), vec![&env]);
        let multiplier = 10u64.pow(decimals);

        // Get voter's token balance
        let voter_balance_stroop: i128 = env.invoke_contract(
            &comp.token,
            &Symbol::new(&env, "balance"),
            vec![&env, voter.into_val(&env)],
        );
        let voter_balance_lumen = (voter_balance_stroop as u64) / multiplier;

        let current_balance = voter_balance_lumen;
        let min_required = comp.min_vote_tokens;
        let has_voted = comp.vote_log.contains_key(voter.clone());
        let can_vote = voting_active && current_balance >= min_required && !has_voted;

        VotingEligibility {
            can_vote,
            has_voted,
            current_balance,
            min_required,
            voting_active,
        }
    }
    /// Get the current contract version
    pub fn version(env: Env) -> String {
        String::from_str(&env, "1.0.1")
    }
    /// Get the current admins of the contract
    pub fn get_admins(env: Env) -> (Option<Address>, Option<Address>) {
        let admin1 = env
            .storage()
            .instance()
            .get::<Symbol, Address>(&Symbol::new(&env, "admin1"));
        let admin2 = env
            .storage()
            .instance()
            .get::<Symbol, Address>(&Symbol::new(&env, "admin2"));
        (admin1, admin2)
    }
    // Internal function to determine the winner without prize distribution
    fn internal_finalize(env: &Env, comp: &mut Competition) {
        // Early check: if no artists, mark as finalized with no winner
        if comp.artists.len() == 0 {
            comp.finalized = true;
            comp.winner = None;
            return;
        }

        // Collect all artists with their votes and sort them
        let mut artist_votes = Vec::new(env);
        for (_, artist_name) in comp.artists.iter() {
            let votes = comp.votes.get(artist_name.clone()).unwrap_or(0);
            artist_votes.push_back((artist_name.clone(), votes));
        }

        // Sort by votes (descending) - manual bubble sort
        let len = artist_votes.len();
        for i in 0..len {
            for j in 0..(len - 1 - i) {
                let current_votes = artist_votes.get(j).unwrap().1;
                let next_votes = artist_votes.get(j + 1).unwrap().1;
                if current_votes < next_votes {
                    let temp = artist_votes.get(j).unwrap();
                    let next = artist_votes.get(j + 1).unwrap();
                    artist_votes.set(j, next);
                    artist_votes.set(j + 1, temp);
                }
            }
        }

        // Set winner (top artist with votes > 0) - handle case where no votes were cast
        if artist_votes.len() > 0 && artist_votes.get(0).unwrap().1 > 0 {
            let winner_symbol = artist_votes.get(0).unwrap().0.clone();
            comp.winner = Some(winner_symbol.clone());

            // Increment competitions_won for the winning artist
            let mut winner_address: Option<Address> = None;
            for (addr, name) in comp.artists.iter() {
                if name == winner_symbol {
                    winner_address = Some(addr);
                    break;
                }
            }
            if let Some(addr) = winner_address {
                let artist_info_key = Symbol::new(env, "artist_info");
                let mut all_info: Map<Address, ArtistInfo> = env
                    .storage()
                    .instance()
                    .get(&artist_info_key)
                    .unwrap_or(Map::new(env));
                if let Some(mut info) = all_info.get(addr.clone()) {
                    info.competitions_won += 1;
                    all_info.set(addr, info);
                    env.storage().instance().set(&artist_info_key, &all_info);
                }
            }
        } else {
            // No votes cast - just mark as finalized with no winner
            comp.winner = None;
        }

        comp.finalized = true;
    }
}
