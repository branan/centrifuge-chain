#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

use codec::HasCompact;
use core::{convert::TryFrom, ops::AddAssign};
use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::UnixTime};
use frame_system::pallet_prelude::*;
use orml_traits::MultiCurrency;
use sp_runtime::{
	traits::{
		AccountIdConversion, AtLeast32BitUnsigned, CheckedAdd, CheckedMul, CheckedSub, One, Zero,
	},
	FixedPointNumber, FixedPointOperand, PerThing, Perquintill, TypeId,
};
use sp_std::vec::Vec;

/// Trait for converting a pool+tranche ID pair to a CurrencyId
///
/// This should be implemented in the runtime to convert from the
/// PoolId and TrancheId types to a CurrencyId that represents that
/// tranche.
///
/// The pool epoch logic assumes that every tranche has a UNIQUE
/// currency, but nothing enforces that. Failure to ensure currency
/// uniqueness will almost certainly cause some wild bugs.
pub trait TrancheToken<T: Config> {
	fn tranche_token(pool: T::PoolId, tranche: T::TrancheId) -> T::CurrencyId;
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct Tranche<Balance> {
	pub interest_per_sec: Perquintill,
	pub min_subordination_ratio: Perquintill,
	pub epoch_supply: Balance,
	pub epoch_redeem: Balance,

	pub debt: Balance,
	pub reserve: Balance,
	pub ratio: Perquintill,
	pub last_updated_interest: u64,
}

#[derive(Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct PoolDetails<AccountId, CurrencyId, EpochId, Balance> {
	pub owner: AccountId,
	pub currency: CurrencyId,
	pub tranches: Vec<Tranche<Balance>>,
	pub current_epoch: EpochId,
	pub last_epoch_closed: u64,
	pub last_epoch_executed: EpochId,
	pub closing_epoch: Option<EpochId>,
	pub max_reserve: Balance,
	pub available_reserve: Balance,
	pub total_reserve: Balance,
}

/// Per-tranche and per-user order details.
#[derive(Clone, Default, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub struct UserOrder<Balance, EpochId> {
	pub supply: Balance,
	pub redeem: Balance,
	pub epoch: EpochId,
}

/// A representation of a tranche identifier that can be used as a storage key
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct TrancheLocator<PoolId, TrancheId> {
	pub pool_id: PoolId,
	pub tranche_id: TrancheId,
}

/// A representation of a pool identifier that can be converted to an account address
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug)]
pub struct PoolLocator<PoolId> {
	pub pool_id: PoolId,
}

impl<PoolId> TypeId for PoolLocator<PoolId> {
	const TYPE_ID: [u8; 4] = *b"pool";
}

/// The result of epoch execution of a given tranch within a pool
#[derive(Encode, Decode, Clone, Eq, PartialEq, RuntimeDebug, Default)]
pub struct EpochDetails<BalanceRatio> {
	pub supply_fulfillment: Perquintill,
	pub redeem_fulfillment: Perquintill,
	pub token_price: BalanceRatio,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use sp_std::convert::TryInto;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Balance: Member
			+ Parameter
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaxEncodedLen
			+ FixedPointOperand
			+ From<u64>
			+ TryInto<u64>;

		/// A fixed-point number which represents the value of
		/// one currency type in terms of another.
		type BalanceRatio: Member
			+ Parameter
			+ Default
			+ Copy
			+ FixedPointNumber<Inner = Self::Balance>;
		type PoolId: Member + Parameter + Default + Copy + HasCompact + MaxEncodedLen;
		type TrancheId: Member
			+ Parameter
			+ Default
			+ Copy
			+ HasCompact
			+ MaxEncodedLen
			+ Into<usize>
			+ TryFrom<usize>;
		type EpochId: Member
			+ Parameter
			+ Default
			+ Copy
			+ HasCompact
			+ MaxEncodedLen
			+ Zero
			+ One
			+ AddAssign;
		type CurrencyId: Parameter + Copy;
		type Tokens: MultiCurrency<
			Self::AccountId,
			Balance = Self::Balance,
			CurrencyId = Self::CurrencyId,
		>;

		/// A conversion from a tranche ID to a CurrencyId
		type TrancheToken: TrancheToken<Self>;
		type Time: UnixTime;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn pool)]
	pub type Pool<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::PoolId,
		PoolDetails<T::AccountId, T::CurrencyId, T::EpochId, T::Balance>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn order)]
	pub type Order<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		TrancheLocator<T::PoolId, T::TrancheId>,
		Blake2_128Concat,
		T::AccountId,
		UserOrder<T::Balance, T::EpochId>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn epoch)]
	pub type Epoch<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		TrancheLocator<T::PoolId, T::TrancheId>,
		Blake2_128Concat,
		T::EpochId,
		EpochDetails<T::BalanceRatio>,
	>;

	#[pallet::storage]
	#[pallet::getter(fn epoch_targets)]
	pub type EpochTargets<T: Config> =
		StorageMap<_, Blake2_128Concat, T::PoolId, Vec<(T::Balance, T::Balance)>>;

	// Pallets use events to inform users when important changes are made.
	// https://substrate.dev/docs/en/knowledgebase/runtime/events
	#[pallet::event]
	#[pallet::metadata(T::AccountId = "AccountId")]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Pool Created. [pool, who]
		PoolCreated(T::PoolId, T::AccountId),
		/// Epoch executed [pool, epoch]
		EpochExecuted(T::PoolId, T::EpochId),
		/// Epoch closed [pool, epoch]
		EpochClosed(T::PoolId, T::EpochId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// A pool with this ID is already in use
		PoolInUse,
		/// Attemppted to create a pool without a juniortranche
		NoJuniorTranche,
		/// Attempted an operation on a pool which does not exist
		NoSuchPool,
		/// Attempted an operation while a pool is closing
		PoolClosing,
		/// An arithmetic overflow occured
		Overflow,
		/// A Tranche ID cannot be converted to an address
		TrancheId,
		/// Closing the epoch now would wipe out the junior tranche
		WipedOut,
		/// The provided solution is not a valid one
		InvalidSolution,
		/// Attempted to solve a pool which is not closing
		NotClosing,
		/// Insufficient currency available for desired operation
		InsufficientCurrency,
		/// Insufficient reserve available for desired operation
		InsufficientReserve,
		/// Subordination Ratio validation failed
		SubordinationRatioViolated,
		/// Generic error for invalid input data provided
		InvalidData,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(100)]
		pub fn create_pool(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			tranches: Vec<(u8, u8)>,
			currency: T::CurrencyId,
			max_reserve: T::Balance,
		) -> DispatchResult {
			let owner = ensure_signed(origin)?;

			// TODO: Ensure owner is authorized to create a pool

			// A single pool ID can only be used by one owner.
			ensure!(!Pool::<T>::contains_key(pool_id), Error::<T>::PoolInUse);

			// At least one tranch must exist, and the last
			// tranche must have an interest rate of 0,
			// indicating that it recieves all remaining
			// equity
			ensure!(
				tranches.last() == Some(&(0, 0)),
				Error::<T>::NoJuniorTranche
			);

			let now = T::Time::now().as_secs();
			let tranches = tranches
				.into_iter()
				.map(|(interest, sub_percent)| {
					const SECS_PER_YEAR: u64 = 365 * 24 * 60 * 60;
					let interest_per_sec =
						Perquintill::from_percent(interest.into()) / SECS_PER_YEAR;
					Tranche {
						interest_per_sec,
						min_subordination_ratio: Perquintill::from_percent(sub_percent.into()),
						epoch_supply: Zero::zero(),
						epoch_redeem: Zero::zero(),

						debt: Zero::zero(),
						reserve: Zero::zero(),
						ratio: Perquintill::zero(),
						last_updated_interest: now,
					}
				})
				.collect();
			Pool::<T>::insert(
				pool_id,
				PoolDetails {
					owner: owner.clone(),
					currency,
					tranches,
					current_epoch: One::one(),
					last_epoch_closed: now,
					last_epoch_executed: Zero::zero(),
					closing_epoch: None,
					max_reserve,
					available_reserve: Zero::zero(),
					total_reserve: Zero::zero(),
				},
			);
			Self::deposit_event(Event::PoolCreated(pool_id, owner));
			Ok(())
		}

		#[pallet::weight(100)]
		pub fn order_supply(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			tranche_id: T::TrancheId,
			amount: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// TODO: Ensure this account is authorized for this tranche
			let (currency, epoch) = {
				let pool = Pool::<T>::try_get(pool_id).map_err(|_| Error::<T>::NoSuchPool)?;
				ensure!(pool.closing_epoch.is_none(), Error::<T>::PoolClosing);
				(pool.currency, pool.current_epoch)
			};
			let tranche = TrancheLocator {
				pool_id,
				tranche_id,
			};
			let pool_account = PoolLocator { pool_id }.into_account();

			Order::<T>::try_mutate(&tranche, &who, |order| -> DispatchResult {
				if amount > order.supply {
					let transfer_amount = amount - order.supply;
					Pool::<T>::try_mutate(pool_id, |pool| {
						let pool = pool.as_mut().ok_or(Error::<T>::NoSuchPool)?;
						let epoch_supply = &mut pool.tranches[tranche_id.into()].epoch_supply;
						*epoch_supply = epoch_supply
							.checked_add(&transfer_amount)
							.ok_or(Error::<T>::Overflow)?;
						T::Tokens::transfer(currency, &who, &pool_account, transfer_amount)
					})?;
				} else if amount < order.supply {
					let transfer_amount = order.supply - amount;
					Pool::<T>::try_mutate(pool_id, |pool| {
						let pool = pool.as_mut().ok_or(Error::<T>::NoSuchPool)?;
						let epoch_supply = &mut pool.tranches[tranche_id.into()].epoch_supply;
						*epoch_supply = epoch_supply
							.checked_sub(&transfer_amount)
							.ok_or(Error::<T>::Overflow)?;
						T::Tokens::transfer(currency, &pool_account, &who, transfer_amount)
					})?;
				}
				order.supply = amount;
				order.epoch = epoch;
				Ok(())
			})
		}

		#[pallet::weight(100)]
		pub fn order_redeem(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			tranche_id: T::TrancheId,
			amount: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// TODO: Ensure this account is authorized for this tranche
			let epoch = {
				let pool = Pool::<T>::try_get(pool_id).map_err(|_| Error::<T>::NoSuchPool)?;
				ensure!(pool.closing_epoch.is_none(), Error::<T>::PoolClosing);
				pool.current_epoch
			};
			let currency = T::TrancheToken::tranche_token(pool_id, tranche_id);
			let tranche = TrancheLocator {
				pool_id,
				tranche_id,
			};
			let pool_account = PoolLocator { pool_id }.into_account();

			Order::<T>::try_mutate(&tranche, &who, |order| -> DispatchResult {
				if amount > order.redeem {
					let transfer_amount = amount - order.redeem;
					Pool::<T>::try_mutate(pool_id, |pool| {
						let pool = pool.as_mut().ok_or(Error::<T>::NoSuchPool)?;
						let epoch_redeem = &mut pool.tranches[tranche_id.into()].epoch_redeem;
						*epoch_redeem = epoch_redeem
							.checked_add(&transfer_amount)
							.ok_or(Error::<T>::Overflow)?;
						T::Tokens::transfer(currency, &who, &pool_account, transfer_amount)
					})?;
				} else if amount < order.redeem {
					let transfer_amount = order.redeem - amount;
					Pool::<T>::try_mutate(pool_id, |pool| {
						let pool = pool.as_mut().ok_or(Error::<T>::NoSuchPool)?;
						let epoch_redeem = &mut pool.tranches[tranche_id.into()].epoch_redeem;
						*epoch_redeem = epoch_redeem
							.checked_sub(&transfer_amount)
							.ok_or(Error::<T>::Overflow)?;
						T::Tokens::transfer(currency, &pool_account, &who, transfer_amount)
					})?;
				}
				order.redeem = amount;
				order.epoch = epoch;
				Ok(())
			})
		}

		#[pallet::weight(100)]
		pub fn close_epoch(origin: OriginFor<T>, pool_id: T::PoolId) -> DispatchResult {
			ensure_signed(origin)?;
			Pool::<T>::try_mutate(pool_id, |pool| {
				let pool = pool.as_mut().ok_or(Error::<T>::NoSuchPool)?;
				ensure!(pool.closing_epoch.is_none(), Error::<T>::PoolClosing);
				let closing_epoch = pool.current_epoch;
				pool.current_epoch += One::one();
				let current_epoch_end = T::Time::now().as_secs();
				pool.last_epoch_closed = current_epoch_end;
				pool.available_reserve = Zero::zero();
				let epoch_reserve = pool.total_reserve;

				// TODO: get NAV
				// For now, assume that nav == 0, so all value is in the reserve
				let nav = Zero::zero();

				let epoch_tranche_prices =
					Self::calculate_tranche_prices(pool_id, nav, epoch_reserve, &mut pool.tranches)
						.ok_or(Error::<T>::Overflow)?;

				if pool
					.tranches
					.iter()
					.all(|tranche| tranche.epoch_supply.is_zero() && tranche.epoch_redeem.is_zero())
				{
					// This epoch is a no-op. Finish executing it.
					for (tranche_id, tranche) in pool.tranches.iter_mut().enumerate() {
						let loc = TrancheLocator {
							pool_id,
							tranche_id: T::TrancheId::try_from(tranche_id)
								.map_err(|_| Error::<T>::TrancheId)?,
						};
						Self::update_tranche_for_epoch(
							loc,
							closing_epoch,
							tranche,
							(Perquintill::zero(), Perquintill::zero()),
							(Zero::zero(), Zero::zero()),
							Zero::zero(),
						)?;
					}
					pool.available_reserve = epoch_reserve;
					pool.last_epoch_executed += One::one();
					Self::deposit_event(Event::EpochExecuted(pool_id, closing_epoch));
					return Ok(());
				}

				// If closing the epoch would wipe out a tranche, the close is invalid.
				// TODO: This should instead put the tranche into an error state
				ensure!(
					!epoch_tranche_prices
						.iter()
						.any(|price| *price == Zero::zero()),
					Error::<T>::WipedOut
				);

				// Redeem orders are denominated in tranche tokens, not in the pool currency.
				// Convert redeem orders to the pool currency and return a list of (supply, redeem) pairs.
				let epoch_targets =
					Self::calculate_epoch_transfers(&epoch_tranche_prices, &pool.tranches)
						.ok_or(Error::<T>::Overflow)?;

				let full_epoch = epoch_targets
					.iter()
					.map(|_| (Perquintill::one(), Perquintill::one()))
					.collect::<Vec<_>>();

				// todo Define/Calculate current tranche values
				// How to include NAV in this
				let current_tranche_values = pool
					.tranches
					.iter()
					.map(|_| Zero::zero())
					.collect::<Vec<_>>();

				if Self::is_epoch_valid(pool, &epoch_targets, &current_tranche_values, &full_epoch)
					.is_ok()
				{
					Self::do_execute_epoch(pool_id, pool, &epoch_targets, &full_epoch)?;
					Self::deposit_event(Event::EpochExecuted(pool_id, closing_epoch));
				} else {
					pool.closing_epoch = Some(closing_epoch);
					EpochTargets::<T>::insert(pool_id, epoch_targets);
					Self::deposit_event(Event::EpochClosed(pool_id, closing_epoch));
				}
				Ok(())
			})
		}

		#[pallet::weight(100)]
		pub fn solve_epoch(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			solution: Vec<(Perquintill, Perquintill)>,
		) -> DispatchResult {
			ensure_signed(origin)?;

			let target = EpochTargets::<T>::try_get(pool_id).map_err(|_| Error::<T>::NoSuchPool)?;
			Pool::<T>::try_mutate(pool_id, |pool| {
				let pool = pool.as_mut().ok_or(Error::<T>::NoSuchPool)?;
				let closing_epoch = pool.closing_epoch.ok_or(Error::<T>::NotClosing)?;

				// todo Define/Calculate current tranche values
				let current_tranche_values = pool
					.tranches
					.iter()
					.map(|_| Zero::zero())
					.collect::<Vec<_>>();

				let epoch_validation_result =
					Self::is_epoch_valid(pool, &target, &current_tranche_values, &solution);

				// Soft error check only for core constraints
				ensure!(
					epoch_validation_result.is_ok()
						|| (epoch_validation_result.err().unwrap()
							!= Error::<T>::InsufficientCurrency.into()),
					Error::<T>::InvalidSolution
				);

				pool.closing_epoch = None;
				Self::do_execute_epoch(pool_id, pool, &target, &solution)?;
				EpochTargets::<T>::remove(pool_id);
				Self::deposit_event(Event::EpochExecuted(pool_id, closing_epoch));
				Ok(())
			})
		}

		// Reserve Operations

		#[pallet::weight(100)]
		pub fn test_payback(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			amount: T::Balance,
		) -> DispatchResult {
			// Internal/pvt call from Coordinator, so no need to check origin on final implementation
			let who = ensure_signed(origin)?;
			let pool_account = PoolLocator { pool_id }.into_account();
			Pool::<T>::try_mutate(pool_id, |pool| {
				let pool = pool.as_mut().ok_or(Error::<T>::NoSuchPool)?;
				pool.total_reserve = pool
					.total_reserve
					.checked_add(&amount)
					.ok_or(Error::<T>::Overflow)?;
				T::Tokens::transfer(pool.currency, &who, &pool_account, amount)?;
				Ok(())
			})
		}

		#[pallet::weight(100)]
		pub fn test_borrow(
			origin: OriginFor<T>,
			pool_id: T::PoolId,
			amount: T::Balance,
		) -> DispatchResult {
			// Internal/pvt call from Coordinator, so no need to check origin on final implementation
			let who = ensure_signed(origin)?;
			let pool_account = PoolLocator { pool_id }.into_account();
			Pool::<T>::try_mutate(pool_id, |pool| {
				let pool = pool.as_mut().ok_or(Error::<T>::NoSuchPool)?;
				pool.total_reserve = pool
					.total_reserve
					.checked_sub(&amount)
					.ok_or(Error::<T>::Overflow)?;
				pool.available_reserve = pool
					.available_reserve
					.checked_sub(&amount)
					.ok_or(Error::<T>::Overflow)?;
				T::Tokens::transfer(pool.currency, &pool_account, &who, amount)?;
				Ok(())
			})
		}
	}

	impl<T: Config> Pallet<T> {
		fn calculate_tranche_prices(
			pool_id: T::PoolId,
			epoch_nav: T::Balance,
			epoch_reserve: T::Balance,
			tranches: &mut [Tranche<T::Balance>],
		) -> Option<Vec<T::BalanceRatio>> {
			let total_assets = epoch_nav.checked_add(&epoch_reserve).unwrap();
			let mut remaining_assets = total_assets;
			let pool_is_zero = total_assets == Zero::zero();
			let last_tranche = tranches.len() - 1;
			tranches
				.iter_mut()
				.enumerate()
				.map(|(tranche_id, tranche)| {
					let currency =
						T::TrancheToken::tranche_token(pool_id, tranche_id.try_into().ok()?);
					let total_issuance = T::Tokens::total_issuance(currency);
					if pool_is_zero || total_issuance == Zero::zero() {
						Some(One::one())
					} else if tranche_id == last_tranche {
						T::BalanceRatio::checked_from_rational(remaining_assets, total_issuance)
					} else {
						Self::update_tranche_debt(tranche)?;
						let tranche_value = tranche.debt.checked_add(&tranche.reserve)?;
						let tranche_value = if tranche_value > remaining_assets {
							remaining_assets = Zero::zero();
							remaining_assets
						} else {
							remaining_assets -= tranche_value;
							tranche_value
						};
						T::BalanceRatio::checked_from_rational(tranche_value, total_issuance)
					}
				})
				.collect()
		}

		fn update_tranche_debt(tranche: &mut Tranche<T::Balance>) -> Option<()> {
			let delta = T::Time::now().as_secs() - tranche.last_updated_interest;
			let interest: T::BalanceRatio = <T::BalanceRatio as One>::one()
				+ T::BalanceRatio::checked_from_rational(
					tranche.interest_per_sec.deconstruct(),
					Perquintill::ACCURACY,
				)?;
			let mut total_interest = T::BalanceRatio::checked_from_integer(One::one())?;
			// TODO: Use a less-bad pow() implementation
			for _ in 0..delta {
				total_interest = interest.checked_mul(&total_interest)?;
			}
			tranche.debt = total_interest.checked_mul_int(tranche.debt)?;
			Some(())
		}

		pub fn calculate_epoch_transfers(
			epoch_tranche_prices: &[T::BalanceRatio],
			tranches: &[Tranche<T::Balance>],
		) -> Option<Vec<(T::Balance, T::Balance)>> {
			epoch_tranche_prices
				.iter()
				.zip(tranches.iter())
				.map(|(price, tranche)| {
					price
						.checked_mul_int(tranche.epoch_redeem)
						.map(|redeem| (tranche.epoch_supply, redeem))
				})
				.collect()
		}

		pub fn is_epoch_valid(
			pool_details: &PoolDetails<T::AccountId, T::CurrencyId, T::EpochId, T::Balance>,
			target: &[(T::Balance, T::Balance)],
			current_tranche_values: &[T::Balance],
			solution: &[(Perquintill, Perquintill)],
		) -> DispatchResult {
			let acc_supply: T::Balance = target.iter().zip(solution.iter()).fold(
				Zero::zero(),
				|sum: T::Balance, (tranche, sol)| {
					sum.checked_add(&sol.0.mul_floor(tranche.0)).unwrap()
				},
			);

			let acc_redeem: T::Balance = target.iter().zip(solution.iter()).fold(
				Zero::zero(),
				|sum: T::Balance, (tranche, sol)| {
					sum.checked_add(&sol.1.mul_floor(tranche.1)).unwrap()
				},
			);

			let currency_available: T::Balance =
				acc_supply.checked_add(&pool_details.total_reserve).unwrap();
			Self::validate_core_constraints(currency_available, acc_redeem)?;

			let new_reserve = currency_available.checked_sub(&acc_redeem).unwrap();

			let tranche_ratios = pool_details
				.tranches
				.iter()
				.map(|tranche| tranche.min_subordination_ratio)
				.collect::<Vec<_>>();

			Self::validate_pool_constraints(
				new_reserve,
				pool_details.max_reserve,
				&tranche_ratios,
				current_tranche_values,
			)
		}

		fn validate_core_constraints(
			currency_available: T::Balance,
			currency_out: T::Balance,
		) -> DispatchResult {
			if currency_out > currency_available {
				Err(Error::<T>::InsufficientCurrency)?
			}
			Ok(())
		}

		fn validate_pool_constraints(
			reserve: T::Balance,
			max_reserve: T::Balance,
			tranche_ratios: &[Perquintill],
			current_tranche_values: &[T::Balance],
		) -> DispatchResult {
			if tranche_ratios.len() != current_tranche_values.len() {
				Err(Error::<T>::InvalidData)?
			}

			if reserve > max_reserve {
				Err(Error::<T>::InsufficientReserve)?
			}

			let mut count = 0;
			for tv in current_tranche_values {
				let acc_sub_value = current_tranche_values
					.iter()
					.skip(count + 1)
					.fold(Zero::zero(), |sum: T::Balance, tvs| {
						sum.checked_add(tvs).unwrap()
					});

				let calc_sub: Perquintill = Perquintill::from_rational(acc_sub_value, *tv);
				if calc_sub < tranche_ratios[count] {
					Err(Error::<T>::SubordinationRatioViolated)?
				}
				count = count + 1;
			}

			Ok(())
		}

		fn do_execute_epoch(
			pool_id: T::PoolId,
			pool: &mut PoolDetails<T::AccountId, T::CurrencyId, T::EpochId, T::Balance>,
			target: &[(T::Balance, T::Balance)],
			solution: &[(Perquintill, Perquintill)],
		) -> DispatchResult {
			pool.last_epoch_executed += One::one();

			let execution: Vec<_> = target
				.iter()
				.copied()
				.zip(solution.iter())
				.map(|((t_supply, t_redeem), (s_supply, s_redeem))| {
					(s_supply.mul_floor(t_supply), s_redeem.mul_floor(t_redeem))
				})
				.collect();

			let total_supply = execution
				.iter()
				.fold(
					Some(Zero::zero()),
					|acc: Option<T::Balance>, (supply, _)| {
						acc.and_then(|acc| acc.checked_add(supply))
					},
				)
				.ok_or(Error::<T>::Overflow)?;

			let total_redeem = execution
				.iter()
				.fold(
					Some(Zero::zero()),
					|acc: Option<T::Balance>, (_, redeem)| {
						acc.and_then(|acc| acc.checked_add(redeem))
					},
				)
				.ok_or(Error::<T>::Overflow)?;

			for (((tranche_id, tranche), solution), execution) in pool
				.tranches
				.iter_mut()
				.enumerate()
				.zip(solution.iter().copied())
				.zip(execution.iter().copied())
			{
				let loc = TrancheLocator {
					pool_id,
					tranche_id: T::TrancheId::try_from(tranche_id)
						.map_err(|_| Error::<T>::TrancheId)?,
				};
				Self::update_tranche_for_epoch(
					loc,
					pool.last_epoch_executed,
					tranche,
					solution,
					execution,
					Zero::zero(), // TODO: Get token price
				)?;
			}

			pool.total_reserve = pool
				.total_reserve
				.checked_add(&total_supply)
				.and_then(|res| res.checked_sub(&total_redeem))
				.ok_or(Error::<T>::Overflow)?;

			Ok(())
		}

		fn update_tranche_for_epoch(
			loc: TrancheLocator<T::PoolId, T::TrancheId>,
			closing_epoch: T::EpochId,
			tranche: &mut Tranche<T::Balance>,
			solution: (Perquintill, Perquintill),
			execution: (T::Balance, T::Balance),
			price: T::BalanceRatio,
		) -> DispatchResult {
			// Update supply/redeem orders for the next epoch based on our execution
			tranche.epoch_supply -= execution.0;
			tranche.epoch_redeem -= execution.1;

			// Compute the tranche tokens that need to be minted or burned based on the execution
			let mint_amount = if price > Zero::zero() {
				price
					.checked_mul_int(execution.0 - execution.1)
					.ok_or(Error::<T>::Overflow)?
			} else {
				Zero::zero()
			};
			let burn_amount = price
				.checked_mul_int(execution.1 - execution.0)
				.ok_or(Error::<T>::Overflow)?;

			let pool_address = PoolLocator {
				pool_id: loc.pool_id,
			}
			.into_account();
			let token = T::TrancheToken::tranche_token(loc.pool_id, loc.tranche_id);
			if mint_amount > burn_amount {
				let tokens_to_mint = mint_amount - burn_amount;
				T::Tokens::deposit(token, &pool_address, tokens_to_mint)?;
			} else if burn_amount > mint_amount {
				let tokens_to_burn = burn_amount - mint_amount;
				T::Tokens::withdraw(token, &pool_address, tokens_to_burn)?;
			}

			// Insert epoch closing information on supply/redeem fulfillment
			let epoch = EpochDetails::<T::BalanceRatio> {
				supply_fulfillment: solution.0,
				redeem_fulfillment: solution.1,
				token_price: price,
			};
			Epoch::<T>::insert(loc, closing_epoch, epoch);
			Ok(())
		}
	}
}
