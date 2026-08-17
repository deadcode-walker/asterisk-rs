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


def check_remote_attach_rejected() -> None:
    with tempfile.TemporaryDirectory(prefix="asterisk-rs-live-attach-") as directory:
        temp = Path(directory)
        cargo_started = temp / "cargo-started"
        write_executable(temp / "docker", "#!/usr/bin/env bash\nexit 0\n")
        write_executable(
            temp / "cargo",
            "#!/usr/bin/env bash\n"
            ": > \"$LIVE_RUNNER_STARTED\"\n"
            "exit 0\n",
        )

        environment = os.environ.copy()
        environment.update(
            {
                "PATH": f"{temp}:{environment['PATH']}",
                "LIVE_RUNNER_STARTED": str(cargo_started),
                "ASTERISK_TEST_ALLOW_MUTATION": "1",
                "ASTERISK_TEST_INSTANCE_MARKER": "owned-fixture",
                "ASTERISK_TEST_BRANCH": "22",
                "ASTERISK_AMI_HOST": "127.0.0.1",
                "ASTERISK_AMI_PORT": "5038",
                "ASTERISK_AMI_USERNAME": "user",
                "ASTERISK_AMI_SECRET": "secret",
                "ASTERISK_ARI_HOST": "127.0.0.1",
                "ASTERISK_ARI_PORT": "8088",
                "ASTERISK_ARI_USERNAME": "user",
                "ASTERISK_ARI_PASSWORD": "secret",
                "ASTERISK_ARI_APP": "app",
                "ASTERISK_TEST_MEDIA_BIND": "127.0.0.1",
                "ASTERISK_TEST_MEDIA_PEER": "127.0.0.1",
            }
        )
        for host_name in ("ASTERISK_AMI_HOST", "ASTERISK_ARI_HOST"):
            environment[host_name] = "192.0.2.10"
            result = subprocess.run(
                [str(ROOT / "scripts/run-live-tests.sh"), "smoke", "attach"],
                cwd=ROOT,
                env=environment,
                capture_output=True,
                text=True,
                timeout=5,
                check=False,
            )
            environment[host_name] = "127.0.0.1"
            if result.returncode == 0:
                raise AssertionError(f"remote cleartext {host_name} attach unexpectedly succeeded")
            if "restricted to an explicit loopback IP address" not in result.stderr:
                raise AssertionError(f"unexpected remote-attach failure: {result.stderr!r}")
            if cargo_started.exists():
                raise AssertionError(
                    "remote attach reached Cargo after credential preflight rejection"
                )



def main() -> None:
    check_signal(signal.SIGINT, 130)
    check_signal(signal.SIGTERM, 143)
    check_remote_attach_rejected()
    print(
        "live runner preserves signal failures, performs cleanup exactly once, "
        "and rejects remote cleartext attach"
    )


if __name__ == "__main__":
    main()
