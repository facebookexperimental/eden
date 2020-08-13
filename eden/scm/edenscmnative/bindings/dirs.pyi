# Copyright (c) Facebook, Inc. and its affiliates.
#
# This software may be used and distributed according to the terms of the
# GNU General Public License version 2.

from typing import Iterable

class dirs(Iterable[str]):
    def __init__(self, paths: Iterable[str]) -> None: ...
    def addpath(self, path: str) -> None: ...
    def delpath(self, path: str) -> None: ...
    def __contains__(self, path: str) -> bool: ...
    def __len__(self) -> int: ...
    def __iter__(self) -> Iterable[str]: ...