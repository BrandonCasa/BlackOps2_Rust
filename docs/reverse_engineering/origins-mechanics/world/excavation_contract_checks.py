"""Synthetic excavation proposal. Explicit draws; no native/RNG/runtime proof."""
from dataclasses import dataclass
import unittest


@dataclass
class Player:
    shovel: bool = False
    upgraded: bool = False
    pickup_origin: tuple | None = None


@dataclass
class Site:
    dug: bool = True
    ready: bool = False


def pickup(player, origin, available=True, owner_matches=True):
    if available and owner_matches and not player.shovel:
        player.shovel = True
        player.pickup_origin = origin
        return False  # world pickup no longer available
    return available


def consume(player, site, count, eligible_use=True):
    # Idempotency is a proposed explicit replacement for one coroutine lifetime.
    if eligible_use and player.shovel and site.ready and not site.dug:
        site.dug, site.ready = True, False
        return count - 1, True
    return count, False


def quota(snow, rain, observed_draw):
    if snow > 0:
        return 0
    return (5 if rain > 0 else 3) + observed_draw


def refill(ordered_sites, count, allowance, maximum=15):
    """Serial immediate-spawn-count model; quest snow exception excluded."""
    spawned = 0
    for site in ordered_sites:
        if site.dug and spawned < allowance and count <= maximum:
            site.dug, site.ready = False, False
            count += 1  # count increments before emergence completes
            spawned += 1
    return count, spawned


class ExcavationProposalCases(unittest.TestCase):
    def test_personal_pickup_consumes_world_pickup(self):
        player = Player()
        available = pickup(player, (1, 2, 3))
        self.assertTrue(player.shovel)
        self.assertFalse(available)
        other = Player()
        self.assertFalse(pickup(other, (1, 2, 3), available))
        self.assertFalse(other.shovel)

    def test_wrong_trigger_owner(self):
        player = Player()
        self.assertTrue(pickup(player, (1, 2, 3), owner_matches=False))
        self.assertFalse(player.shovel)

    def test_already_owned_does_not_consume_another_pickup(self):
        player = Player(True)
        self.assertTrue(pickup(player, (1, 2, 3)))

    def test_no_shovel_does_not_deplete(self):
        site = Site(False, True)
        self.assertEqual(consume(Player(), site, 1), (1, False))
        self.assertFalse(site.dug)

    def test_dig_preserves_personal_capability(self):
        player, site = Player(True), Site(False, True)
        self.assertEqual(consume(player, site, 1), (0, True))
        self.assertTrue(player.shovel)
        self.assertTrue(site.dug)

    def test_repeated_use_idempotency_is_proposal(self):
        player, site = Player(True), Site(False, True)
        count, _ = consume(player, site, 1)
        self.assertEqual(consume(player, site, count), (0, False))

    def test_external_eligibility_required(self):
        self.assertEqual(consume(Player(True), Site(False, True), 1, False),
                         (1, False))

    def test_weather_priority_and_explicit_draw(self):
        self.assertEqual(quota(0, 0, 2), 5)
        self.assertEqual(quota(0, 1, 2), 7)
        self.assertEqual(quota(1, 1, 2), 0)

    def test_equal_limit_admits_one_under_serial_model(self):
        self.assertEqual(refill([Site(), Site()], 15, 2), (16, 1))
        self.assertEqual(refill([Site()], 16, 1), (16, 0))

    def test_quota_and_order(self):
        sites = [Site(), Site(False, True), Site()]
        self.assertEqual(refill(sites, 1, 1), (2, 1))
        self.assertFalse(sites[0].dug)
        self.assertTrue(sites[2].dug)

    def test_emerging_count_not_ready(self):
        site = Site()
        self.assertEqual(refill([site], 0, 1), (1, 1))
        self.assertEqual(consume(Player(True), site, 1), (1, False))
        site.ready = True
        self.assertEqual(consume(Player(True), site, 1), (0, True))

    def test_disconnect_returns_original_pickup_location(self):
        player = Player()
        pickup(player, (1, 2, 3))
        disconnect_position = (999, 999, 999)
        self.assertNotEqual(player.pickup_origin, disconnect_position)
        self.assertEqual(player.pickup_origin, (1, 2, 3))


if __name__ == "__main__":
    unittest.main()
