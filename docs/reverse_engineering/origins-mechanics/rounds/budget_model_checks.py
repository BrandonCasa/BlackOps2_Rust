"""Synthetic explanatory models, NOT an original GSC VM or gameplay tests.

Exact rational arithmetic deliberately avoids assuming T6 float precision. Both
division candidates assume positive int() conversion truncates toward zero.
"""

from fractions import Fraction
import unittest


def budget(round_number, players, division="integer", base=24, per_player=6):
    if not 1 <= round_number <= 255 or not 1 <= players <= 4:
        raise ValueError("outside the explanatory model's selected domain")
    if division == "integer":
        multiplier = Fraction(round_number // 5)
    elif division == "real":
        multiplier = Fraction(round_number, 5)
    else:
        raise ValueError("unknown division candidate")
    multiplier = max(Fraction(1), multiplier)
    if round_number >= 10:
        multiplier *= Fraction(3 * round_number, 20)
    contribution = Fraction(1, 2) if players == 1 else Fraction(players - 1)
    before_early = base + int(contribution * per_player * multiplier)
    early = (Fraction(1, 4), Fraction(3, 10), Fraction(1, 2),
             Fraction(7, 10), Fraction(9, 10))
    return int(before_early * early[round_number - 1]) if round_number < 6 else before_early


def replay(initial, events):
    """Project-owned ledger fixture: event order is input, not a GSC scheduler."""
    pending, counted, replacements, admissions = initial, 0, 0, 0
    for event in events:
        if event == "failed_spawn":
            pass
        elif event == "spawn":
            if pending <= 0:
                raise ValueError("fixture admission without pending budget")
            pending -= 1
            counted += 1
            admissions += 1
        elif event == "replacement":
            if counted <= 0:
                raise ValueError("fixture replacement without counted entity")
            counted -= 1
            pending += 1
            replacements += 1
        elif event == "remove":
            if counted <= 0:
                raise ValueError("fixture removal without counted entity")
            counted -= 1
        else:
            raise ValueError("unknown fixture event")
    return pending, counted, replacements, admissions


class BudgetChecks(unittest.TestCase):
    def test_early_roster_table(self):
        expected = ((6, 7, 9, 10), (8, 9, 10, 12), (13, 15, 18, 21),
                    (18, 21, 25, 29), (24, 27, 32, 37))
        for round_number, row in enumerate(expected, 1):
            for division in ("integer", "real"):
                self.assertEqual(tuple(budget(round_number, p, division) for p in range(1, 5)), row)

    def test_division_counterexamples(self):
        cases = ((6, (27, 30, 36, 42), (27, 31, 38, 45)),
                 (9, (27, 30, 36, 42), (29, 34, 45, 56)),
                 (11, (33, 43, 63, 83), (34, 45, 67, 89)))
        for round_number, integer_row, real_row in cases:
            self.assertEqual(tuple(budget(round_number, p) for p in range(1, 5)), integer_row)
            self.assertEqual(tuple(budget(round_number, p, "real") for p in range(1, 5)), real_row)

    def test_threshold_ten(self):
        for division in ("integer", "real"):
            self.assertEqual(tuple(budget(10, p, division) for p in range(1, 5)), (33, 42, 60, 78))

    def test_full_round_budget_exceeds_concurrent_capacity(self):
        self.assertGreater(budget(10, 4), 24)
        self.assertGreater(budget(255, 1), 24)

    def test_truncation_location_matters(self):
        # Truncating a solo half-player coefficient to zero before multiplying
        # would incorrectly remove the entire contribution.
        self.assertEqual(budget(6, 1), 27)
        self.assertNotEqual(budget(6, 1), 24 + int(Fraction(1, 2)) * 6)
        # The callback's early reduction includes the base as well as the addition.
        self.assertEqual(budget(1, 2), 7)
        self.assertNotEqual(budget(1, 2), 24 + int(Fraction(6, 4)))

    def test_monotonic_selected_domain(self):
        for division in ("integer", "real"):
            for p in range(1, 5):
                values = [budget(r, p, division) for r in range(1, 256)]
                self.assertEqual(values, sorted(values))
            for r in range(1, 256):
                values = [budget(r, p, division) for p in range(1, 5)]
                self.assertEqual(values, sorted(values))

    def test_snapshot_is_not_midround_recalculation(self):
        chosen = budget(1, 1)
        self.assertEqual(chosen, 6)
        self.assertNotEqual(chosen, budget(1, 4))

    def test_model_scope_errors(self):
        for r, p in ((0, 1), (256, 1), (1, 0), (1, 5)):
            with self.assertRaises(ValueError):
                budget(r, p)
        with self.assertRaises(ValueError):
            budget(1, 1, "guessed")

    def test_failure_preserves_work(self):
        self.assertEqual(replay(1, ("failed_spawn",)), (1, 0, 0, 0))

    def test_replacement_can_exceed_initial_admissions(self):
        ordinary = ("spawn", "remove")
        replacement = ("spawn", "replacement", "spawn", "remove")
        self.assertEqual(replay(1, ordinary), (0, 0, 0, 1))
        self.assertEqual(replay(1, replacement), (0, 0, 1, 2))
        self.assertEqual(replay(1, replacement), replay(1, replacement))

    def test_admission_is_not_completion(self):
        self.assertEqual(replay(1, ("spawn",))[:2], (0, 1))

    def test_replay_rejects_impossible_fixture_events(self):
        for events in (("replacement",), ("remove",), ("bogus",), ("spawn", "spawn")):
            with self.assertRaises(ValueError):
                replay(1, events)


if __name__ == "__main__":
    unittest.main()
