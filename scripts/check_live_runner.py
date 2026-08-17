#!/usr/bin/env python3

from __future__ import annotations

import os
import signal
import subprocess
import tempfile
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parent.parent


def write_executable(path: Path, content: str) -> None:
    path.write_text(content, encoding="utf-8")
    path.chmod(0o755)


def check_signal(signum: signal.Signals, expected_status: int) -> None:
    with tempfile.TemporaryDirectory(prefix="asterisk-rs-live-runner-") as directory:
        temp = Path(directory)
        commands = temp / "commands.log"
        started = temp / "cargo-started"

        write_executable(
            temp / "docker",
            "#!/usr/bin/env bash\n"
            "printf '%s\\n' \"$*\" >> \"$LIVE_RUNNER_COMMANDS\"\n"
            "if [[ ${1:-} == inspect ]]; then echo 172.18.0.2; fi\n"
            "if [[ $* == *' exec -T asterisk '* ]]; then echo 172.17.0.1; fi\n",
        )
        write_executable(temp / "python3", "#!/usr/bin/env bash\nexit 0\n")
        write_executable(
            temp / "cargo",
            "#!/usr/bin/env bash\n"
            "if [[ ${1:-} == test ]]; then\n"
            "    echo '1 tests, 0 benchmarks'\n"
            "    exit 0\n"
            "fi\n"
            ": > \"$LIVE_RUNNER_STARTED\"\n"
            "while :; do sleep 1; done\n",
        )

        environment = os.environ.copy()
        environment["PATH"] = f"{temp}:{environment['PATH']}"
        environment["LIVE_RUNNER_COMMANDS"] = str(commands)
        environment["LIVE_RUNNER_STARTED"] = str(started)
        process = subprocess.Popen(
            [str(ROOT / "scripts/run-live-tests.sh"), "smoke", "compose"],
            cwd=ROOT,
            env=environment,
            start_new_session=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        try:
            deadline = time.monotonic() + 5
            while not started.exists() and process.poll() is None:
                if time.monotonic() >= deadline:
                    raise AssertionError("live runner did not reach the test command")
                time.sleep(0.01)
            if process.poll() is not None:
                stdout, stderr = process.communicate()
                raise AssertionError(
                    f"live runner exited before the test command: {process.returncode}; "
                    f"stdout={stdout!r}, stderr={stderr!r}"
                )
            os.killpg(process.pid, signum)
            stdout, stderr = process.communicate(timeout=5)
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGKILL)
                process.wait()

        if process.returncode != expected_status:
            raise AssertionError(
                f"{signum.name} returned {process.returncode}, expected {expected_status}; "
                f"stdout={stdout!r}, stderr={stderr!r}"
            )
        recorded = commands.read_text(encoding="utf-8").splitlines()
        cleanup = [line for line in recorded if " down --volumes --remove-orphans" in line]
        if len(cleanup) != 1:
            raise AssertionError(f"expected exactly one Compose cleanup, observed {recorded!r}")



def main() -> None:
    check_signal(signal.SIGINT, 130)
    check_signal(signal.SIGTERM, 143)
    print("live runner preserves signal failures and performs cleanup exactly once")


if __name__ == "__main__":
    main()
