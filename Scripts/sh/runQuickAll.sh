#!/bin/env -S sh -e  # Run script with POSIX shell, exit immediately on error

# SPDX-FileCopyrightText: 2024 gluesniffler <159397573+gluesniffler@users.noreply.github.com>
# SPDX-FileCopyrightText: 2025 Aiden <28298836+Aidenkrz@users.noreply.github.com>
#
# SPDX-License-Identifier: AGPL-3.0-or-later


cd -- "$(dirname -- "$0")" # Change working directory to script location

sh -e runQuickServer.sh &
sh -e runQuickClient.sh

exit
