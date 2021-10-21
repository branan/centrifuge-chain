use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use primitives_tokens::CurrencyId;
use sp_runtime::traits::{One, Zero};
use sp_runtime::Perquintill;

#[test]
fn core_constraints_currency_available_cant_cover_redemptions() {
	new_test_ext().execute_with(|| {
		let tranches: Vec<_> = std::iter::repeat(Tranche {
			epoch_redeem: 10,
			..Default::default()
		})
		.take(4)
		.collect();

		let epoch_tranches = tranches
			.iter()
			.zip(vec![80, 20, 5, 5]) // no IntoIterator for arrays, so we use a vec here. Meh.
			.map(|(tranche, value)| EpochExecutionTranche {
				value,
				price: One::one(),
				supply: tranche.epoch_supply,
				redeem: tranche.epoch_redeem,
			})
			.collect();

		let pool = &PoolDetails {
			owner: Zero::zero(),
			currency: CurrencyId::Usd,
			tranches,
			current_epoch: Zero::zero(),
			last_epoch_closed: 0,
			last_epoch_executed: Zero::zero(),
			closing_epoch: None,
			max_reserve: 40,
			available_reserve: Zero::zero(),
			total_reserve: 39,
			fake_nav: 0,
		};

		let epoch = EpochExecutionInfo {
			nav: 0,
			reserve: pool.total_reserve,
			tranches: epoch_tranches,
		};

		let full_solution = pool
			.tranches
			.iter()
			.map(|_| (Perquintill::one(), Perquintill::one()))
			.collect::<Vec<_>>();

		assert_noop!(
			TinlakeInvestorPool::is_epoch_valid(pool, &epoch, &full_solution),
			Error::<Test>::InsufficientCurrency
		);
	});
}

#[test]
fn pool_constraints_pool_reserve_above_max_reserve() {
	new_test_ext().execute_with(|| {
		let tranche_a = Tranche {
			epoch_supply: 10,
			epoch_redeem: 10,
			..Default::default()
		};
		let tranche_b = Tranche {
			epoch_supply: Zero::zero(),
			epoch_redeem: 10,
			..Default::default()
		};
		let tranche_c = Tranche {
			epoch_supply: Zero::zero(),
			epoch_redeem: 10,
			..Default::default()
		};
		let tranche_d = Tranche {
			epoch_supply: Zero::zero(),
			epoch_redeem: 10,
			..Default::default()
		};
		let tranches = vec![tranche_a, tranche_b, tranche_c, tranche_d];
		let epoch_tranches = tranches
			.iter()
			.zip(vec![80, 20, 15, 15]) // no IntoIterator for arrays, so we use a vec here. Meh.
			.map(|(tranche, value)| EpochExecutionTranche {
				value,
				price: One::one(),
				supply: tranche.epoch_supply,
				redeem: tranche.epoch_redeem,
			})
			.collect();

		let pool = &PoolDetails {
			owner: Zero::zero(),
			currency: CurrencyId::Usd,
			tranches,
			current_epoch: Zero::zero(),
			last_epoch_closed: 0,
			last_epoch_executed: Zero::zero(),
			closing_epoch: None,
			max_reserve: 5,
			available_reserve: Zero::zero(),
			total_reserve: 40,
			fake_nav: 90,
		};

		let epoch = EpochExecutionInfo {
			nav: 90,
			reserve: pool.total_reserve,
			tranches: epoch_tranches,
		};

		let full_solution = pool
			.tranches
			.iter()
			.map(|_| (Perquintill::one(), Perquintill::one()))
			.collect::<Vec<_>>();

		assert_noop!(
			TinlakeInvestorPool::is_epoch_valid(pool, &epoch, &full_solution),
			Error::<Test>::InsufficientReserve
		);
	});
}

#[test]
fn pool_constraints_tranche_violates_sub_ratio() {
	new_test_ext().execute_with(|| {
		let tranche_a = Tranche {
			min_subordination_ratio: Perquintill::from_float(0.4), // Violates constraint here
			epoch_supply: 100,
			epoch_redeem: Zero::zero(),
			..Default::default()
		};
		let tranche_b = Tranche {
			min_subordination_ratio: Perquintill::from_float(0.5),
			epoch_supply: Zero::zero(),
			epoch_redeem: 20,
			..Default::default()
		};
		let tranche_c = Tranche {
			min_subordination_ratio: Perquintill::from_float(0.5),
			epoch_supply: Zero::zero(),
			epoch_redeem: Zero::zero(),
			..Default::default()
		};
		let tranche_d = Tranche {
			min_subordination_ratio: Perquintill::zero(),
			epoch_supply: Zero::zero(),
			epoch_redeem: Zero::zero(),
			..Default::default()
		};
		let tranches = vec![tranche_a, tranche_b, tranche_c, tranche_d];

		let epoch_tranches = tranches
			.iter()
			.zip(vec![80, 20, 5, 5]) // no IntoIterator for arrays, so we use a vec here. Meh.
			.map(|(tranche, value)| EpochExecutionTranche {
				value,
				price: One::one(),
				supply: tranche.epoch_supply,
				redeem: tranche.epoch_redeem,
			})
			.collect();

		let pool = &PoolDetails {
			owner: Zero::zero(),
			currency: CurrencyId::Usd,
			tranches,
			current_epoch: Zero::zero(),
			last_epoch_closed: 0,
			last_epoch_executed: Zero::zero(),
			closing_epoch: None,
			max_reserve: 150,
			available_reserve: Zero::zero(),
			total_reserve: 50,
			fake_nav: 0,
		};

		let epoch = EpochExecutionInfo {
			nav: 0,
			reserve: pool.total_reserve,
			tranches: epoch_tranches,
		};

		let full_solution = pool
			.tranches
			.iter()
			.map(|_| (Perquintill::one(), Perquintill::one()))
			.collect::<Vec<_>>();

		assert_noop!(
			TinlakeInvestorPool::is_epoch_valid(pool, &epoch, &full_solution),
			Error::<Test>::SubordinationRatioViolated
		);
	});
}

#[test]
fn pool_constraints_pass() {
	new_test_ext().execute_with(|| {
		let tranche_a = Tranche {
			min_subordination_ratio: Perquintill::from_float(0.2),
			epoch_supply: 100,
			epoch_redeem: Zero::zero(),
			..Default::default()
		};
		let tranche_b = Tranche {
			min_subordination_ratio: Perquintill::from_float(0.5),
			epoch_supply: Zero::zero(),
			epoch_redeem: 30,
			..Default::default()
		};
		let tranche_c = Tranche {
			min_subordination_ratio: Perquintill::from_float(0.5),
			epoch_supply: Zero::zero(),
			epoch_redeem: Zero::zero(),
			..Default::default()
		};
		let tranche_d = Tranche {
			min_subordination_ratio: Perquintill::zero(),
			epoch_supply: Zero::zero(),
			epoch_redeem: Zero::zero(),
			..Default::default()
		};
		let tranches = vec![tranche_a, tranche_b, tranche_c, tranche_d];

		let epoch_tranches = tranches
			.iter()
			.zip(vec![80, 70, 35, 20]) // no IntoIterator for arrays, so we use a vec here. Meh.
			.map(|(tranche, value)| EpochExecutionTranche {
				value,
				price: One::one(),
				supply: tranche.epoch_supply,
				redeem: tranche.epoch_redeem,
			})
			.collect();

		let pool = &PoolDetails {
			owner: Zero::zero(),
			currency: CurrencyId::Usd,
			tranches,
			current_epoch: Zero::zero(),
			last_epoch_closed: 0,
			last_epoch_executed: Zero::zero(),
			closing_epoch: None,
			max_reserve: 150,
			available_reserve: Zero::zero(),
			total_reserve: 50,
			fake_nav: 145,
		};

		let epoch = EpochExecutionInfo {
			nav: 145,
			reserve: pool.total_reserve,
			tranches: epoch_tranches,
		};

		let full_solution = pool
			.tranches
			.iter()
			.map(|_| (Perquintill::one(), Perquintill::one()))
			.collect::<Vec<_>>();

		assert_ok!(TinlakeInvestorPool::is_epoch_valid(
			pool,
			&epoch,
			&full_solution
		));
	});
}

#[test]
fn epoch() {
	new_test_ext().execute_with(|| {
		let tin_investor = Origin::signed(0);
		let drop_investor = Origin::signed(1);
		let pool_owner = Origin::signed(2);
		let borrower = Origin::signed(3);
		let pool_account = Origin::signed(PoolLocator { pool_id: 0 }.into_account());

		// Initialize pool with initial investments
		assert_ok!(TinlakeInvestorPool::create_pool(
			pool_owner.clone(),
			0,
			vec![(10, 10), (0, 0)],
			CurrencyId::Usd,
			10_000 * CURRENCY
		));
		assert_ok!(TinlakeInvestorPool::order_supply(
			tin_investor.clone(),
			0,
			1,
			500 * CURRENCY
		));
		assert_ok!(TinlakeInvestorPool::order_supply(
			drop_investor.clone(),
			0,
			0,
			500 * CURRENCY
		));
		assert_ok!(TinlakeInvestorPool::close_epoch(pool_owner.clone(), 0));
		assert_ok!(Tokens::transfer(
			pool_account.clone(),
			0,
			CurrencyId::Tranche(0, 1),
			500 * CURRENCY
		));
		assert_ok!(Tokens::transfer(
			pool_account.clone(),
			1,
			CurrencyId::Tranche(0, 0),
			500 * CURRENCY
		));

		let pool = TinlakeInvestorPool::pool(0).unwrap();
		assert_eq!(pool.tranches[0].debt, 0);
		assert_eq!(pool.tranches[0].reserve, 500 * CURRENCY);
		assert_eq!(pool.tranches[0].ratio, Perquintill::from_float(0.5));
		assert_eq!(pool.tranches[1].debt, 0);
		assert_eq!(pool.tranches[1].reserve, 500 * CURRENCY);
		assert_eq!(pool.available_reserve, 1000 * CURRENCY);
		assert_eq!(pool.total_reserve, 1000 * CURRENCY);

		// Borrow some money
		next_block();
		assert_ok!(TinlakeInvestorPool::test_borrow(
			borrower.clone(),
			0,
			500 * CURRENCY
		));

		let pool = TinlakeInvestorPool::pool(0).unwrap();
		assert_eq!(pool.tranches[0].debt, 250 * CURRENCY);
		assert_eq!(pool.tranches[0].reserve, 250 * CURRENCY);
		assert_eq!(pool.tranches[1].debt, 250 * CURRENCY);
		assert_eq!(pool.tranches[1].reserve, 250 * CURRENCY);
		assert_eq!(pool.available_reserve, 500 * CURRENCY);
		assert_eq!(pool.total_reserve, 500 * CURRENCY);

		// Repay (with made up interest) after a month.
		next_block_after(60 * 60 * 24 * 30);
		assert_ok!(TinlakeInvestorPool::test_nav_up(
			borrower.clone(),
			0,
			10 * CURRENCY
		));
		assert_ok!(TinlakeInvestorPool::test_payback(
			borrower.clone(),
			0,
			510 * CURRENCY
		));

		let pool = TinlakeInvestorPool::pool(0).unwrap();
		assert_eq!(pool.tranches[0].debt, 0);
		assert!(pool.tranches[0].reserve > 500 * CURRENCY); // there's interest in here now
		assert_eq!(pool.tranches[1].debt, 0);
		assert_eq!(pool.tranches[1].reserve, 500 * CURRENCY); // not yet rebalanced
		assert_eq!(pool.available_reserve, 500 * CURRENCY);
		assert_eq!(pool.total_reserve, 1010 * CURRENCY);

		// DROP investor tries to redeem
		next_block();
		assert_ok!(TinlakeInvestorPool::order_redeem(
			drop_investor.clone(),
			0,
			0,
			250 * CURRENCY
		));
		assert_ok!(TinlakeInvestorPool::close_epoch(pool_owner.clone(), 0));

		let pool = TinlakeInvestorPool::pool(0).unwrap();
		// assert_eq!(pool.tranches[0].epoch_redeem, 0);
		assert_eq!(pool.tranches[0].debt, 0);
		assert!(pool.tranches[0].reserve > 250 * CURRENCY); // there's interest in here now
		assert_eq!(pool.tranches[1].debt, 0);
		assert!(pool.tranches[1].reserve > 500 * CURRENCY); // not yet rebalanced
		assert_eq!(pool.available_reserve, pool.total_reserve);
		assert!(pool.total_reserve > 750 * CURRENCY);
		assert!(pool.total_reserve < 800 * CURRENCY);
	});
}
