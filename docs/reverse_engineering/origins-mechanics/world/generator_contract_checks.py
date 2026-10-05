"""Synthetic proposal checks; not a game/VM implementation or fidelity proof."""
import unittest
from fractions import Fraction


def participant(valid, dx, dy, dz):
    return valid and dx * dx + dy * dy < 48400 and dz > -20


def increment(inside, roster, solo=False, recapture=False, target=False,
              attackers=0, attacked=False):
    if recapture and target:
        return Fraction(-1, 4) if attackers else (1 if attacked else 0)
    if target:
        return 1
    if inside:
        return Fraction(5, 6) if solo else Fraction(inside, roster)
    return Fraction(-1, 2)


def update(progress, delta, attacked=False):
    next_progress = max(0, min(100, progress + delta))
    # The original endpoint branch is reached only following a changed raw sum.
    ended = delta != 0 and (next_progress == 0 or
                            (next_progress == 100 and not attacked))
    return next_progress, ended


class GeneratorProposalCases(unittest.TestCase):
    def test_strict_radius(self):
        self.assertTrue(participant(True, 219, 0, 0))
        self.assertFalse(participant(True, 220, 0, 0))

    def test_strict_height_and_validity(self):
        self.assertFalse(participant(True, 0, 0, -20))
        self.assertTrue(participant(True, 0, 0, -19))
        self.assertTrue(participant(True, 0, 0, 10000))
        self.assertFalse(participant(False, 0, 0, 0))

    def test_roster_cost_examples(self):
        self.assertEqual([200 * n for n in (1, 2, 3, 4)], [200, 400, 600, 800])

    def test_ordinary_rates(self):
        self.assertEqual(increment(1, 1, solo=True), Fraction(5, 6))
        self.assertEqual(increment(1, 4), Fraction(1, 4))
        self.assertEqual(increment(4, 4), 1)

    def test_leave_and_reenter(self):
        p, ended = update(50, increment(0, 4))
        self.assertEqual((p, ended), (Fraction(99, 2), False))
        self.assertEqual(update(p, increment(4, 4)), (Fraction(101, 2), False))

    def test_zero_and_restart(self):
        self.assertEqual(update(Fraction(1, 4), increment(0, 1)), (0, True))
        self.assertEqual(update(0, increment(1, 1)), (1, False))

    def test_initial_empty_zero_delta_guard(self):
        self.assertEqual(update(0, 0), (0, False))
        # Negative raw delta reaches the branch even if clamping retains zero.
        self.assertEqual(update(0, increment(0, 1)), (0, True))

    def test_completion_attacked_guard(self):
        self.assertEqual(update(99, 1), (100, True))
        self.assertEqual(update(99, 1, attacked=True), (100, False))

    def test_recapture_ignores_occupancy(self):
        self.assertEqual(increment(4, 4, recapture=True, target=True, attackers=1),
                         Fraction(-1, 4))
        self.assertEqual(increment(0, 4, recapture=True, target=True, attacked=True), 1)
        self.assertEqual(increment(4, 4, recapture=True, target=True), 0)

    def test_target_recovery_without_event(self):
        self.assertEqual(increment(0, 4, target=True), 1)


if __name__ == "__main__":
    unittest.main()
