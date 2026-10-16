"""Evidence contracts are specific to this route; setup acknowledgements are never passes."""
from dataclasses import dataclass, asdict
import json
import re
from pathlib import Path
from client import ValidationFailure


@dataclass(frozen=True)
class Contract:
    id: str
    source: str
    function: str
    prerequisites: tuple[str, ...]
    action: str
    intermediate: tuple[str, ...]
    outcome: str
    timeout: str
    evidence: tuple[str, ...]
    assistance: tuple[str, ...] = ()

    def record(self, root: Path):
        result = asdict(self)
        path = root / self.source
        if not path.is_file():
            raise ValidationFailure(f"Required source contract is missing: {path}")
        result["source_path"] = str(path)
        if not re.search(r"^" + re.escape(self.function) + r"\s*\(", path.read_text(encoding="utf-8"), re.MULTILINE):
            raise ValidationFailure(f"Source function {self.function} is missing from {path}")
        return result


FOUNDATION = (
    Contract("shovel", "source/server-tomb/maps/mp/zm_tomb_dig.gsc", "shovel_unitrigger_think", ("Original session ready", "no shovel owned"),
             "Aim at an existing shovel and hold interact", ("interaction focus matches shovel", "native interaction accepted"),
             "player owns shovel", "30 simulation seconds", ("command receipt", "before/after ownership", "interaction event", "capture"), ("god mode", "points", "ammo")),
    Contract("combat-kill", "source/server-patch/maps/mp/zombies/_zm_spawner.gsc", "zombie_death_event", ("native admitted actor", "actor emerged and alive"),
             "Aim and fire the equipped weapon", ("attributed damage to the selected actor",),
             "native player-attributed EnemyDeath for that actor", "30 simulation seconds per combat action", ("hit and death events", "actor identity", "command receipts", "capture"), ("god mode", "ammo")),
)


def write_contracts(report, research, contracts):
    (report / "contracts.json").write_text(json.dumps([contract.record(research) for contract in contracts], indent=2), encoding="utf-8")
