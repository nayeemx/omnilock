import importlib.util
import os
import re
import sys

REPO = r"D:\projects\code\omnilock\omnilock-bio\reference\python-validity"
SENSOR_DIR = os.path.join(REPO, "validitysensor")
OUTDIR = r"D:\projects\code\omnilock\omnilock-bio\resources"
os.makedirs(OUTDIR, exist_ok=True)

sys.path.insert(0, SENSOR_DIR)
sys.path.insert(0, REPO)


def load_pkg_module(name):
    # Load as "<pkg>.<name>" so relative imports (from .util import ...) resolve.
    spec = importlib.util.spec_from_file_location(
        "validitysensor." + name, os.path.join(SENSOR_DIR, name + ".py")
    )
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def write(name, data):
    p = os.path.join(OUTDIR, name)
    with open(p, "wb") as f:
        f.write(data)
    print("%-32s %8d bytes" % (name, len(data)))


# blobs_97 / blobs_d51 only depend on util (no pyusb).
blobs_97 = load_pkg_module("blobs_97")
blobs_d51 = load_pkg_module("blobs_d51")

write("init_hardcoded.bin", blobs_97.init_hardcoded)
write("init_hardcoded_clean_slate.bin", blobs_97.init_hardcoded_clean_slate)
write("db_write_enable.bin", blobs_97.db_write_enable)
write("reset_blob_d51.bin", blobs_d51.reset_blob)


# tls.py imports pyusb; pull the three constants straight from the source text.
src = open(os.path.join(SENSOR_DIR, "tls.py"), encoding="utf-8").read()


def hexconst(name):
    m = re.search(r"^%s = unhexlify\('([0-9a-fA-F]+)'\)" % name, src, re.M)
    assert m, name
    return bytes.fromhex(m.group(1))


write("password_hardcoded.bin", hexconst("password_hardcoded"))
write("gwk_sign_hardcoded.bin", hexconst("gwk_sign_hardcoded"))

m = re.search(r"crt_hardcoded = unhex\('''(.*?)'''\)", src, re.S)
assert m, "crt_hardcoded"
write("crt_hardcoded.bin", bytes.fromhex(re.sub(r"\W", "", m.group(1))))