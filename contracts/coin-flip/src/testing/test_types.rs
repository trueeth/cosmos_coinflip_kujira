use crate::types::Streak;

#[cfg(test)]
#[test]
fn test_streak_reward() {
    use cosmwasm_std::Uint128;

    use crate::types::StreakReward;

    let streak_reward = StreakReward::new(1, Uint128::new(100));
    assert_eq!(
        streak_reward,
        StreakReward {
            streak: 1,
            reward: Uint128::new(100)
        }
    );
}

#[test]
fn test_streak_update() {
    let mut streak = Streak::new(true);
    assert_eq!(
        streak,
        Streak {
            amount: 1,
            result: true
        }
    );

    streak.update(true);
    assert_eq!(
        streak,
        Streak {
            amount: 2,
            result: true
        }
    );

    streak.update(false);
    assert_eq!(
        streak,
        Streak {
            amount: 1,
            result: false
        }
    );
}
