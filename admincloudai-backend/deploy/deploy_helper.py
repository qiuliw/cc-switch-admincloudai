"""远程部署辅助:paramiko 执行命令 / SFTP 上传。用法:
python deploy_helper.py run "命令"
python deploy_helper.py put 本地路径 远程路径
"""
import sys
import warnings

import paramiko

warnings.filterwarnings("ignore")

HOST = "115.159.196.148"
USER = "root"
PASSWORD = "134679Zxc@"  # noqa: S105 - 用户提供的临时凭据


def connect() -> paramiko.SSHClient:
    client = paramiko.SSHClient()
    client.set_missing_host_key_policy(paramiko.AutoAddPolicy())
    client.connect(
        HOST,
        username=USER,
        password=PASSWORD,
        timeout=20,
        banner_timeout=20,
        auth_timeout=20,
    )
    return client


def main() -> None:
    mode = sys.argv[1]
    client = connect()
    try:
        if mode == "run":
            _, stdout, stderr = client.exec_command(sys.argv[2], timeout=580)
            out = stdout.read().decode("utf-8", "replace")
            err = stderr.read().decode("utf-8", "replace")
            code = stdout.channel.recv_exit_status()
            if out:
                print(out, end="")
            if err:
                print(err, end="", file=sys.stderr)
            sys.exit(code)
        elif mode == "put":
            sftp = client.open_sftp()
            sftp.put(sys.argv[2], sys.argv[3])
            sftp.close()
            print(f"uploaded {sys.argv[2]} -> {sys.argv[3]}")
        else:
            sys.exit(f"unknown mode: {mode}")
    finally:
        client.close()


if __name__ == "__main__":
    main()
