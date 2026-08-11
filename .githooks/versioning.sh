#!/bin/sh

set -eu

ROOT=${ROOT:-$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)}

fail() {
    printf '%s\n' "versioning: $*" >&2
    exit 1
}

read_version() {
    version=$(cat "$ROOT/VERSION") || fail "cannot read VERSION"
    [ -n "$version" ] || fail "VERSION is empty"
    printf '%s\n' "$version"
}

increment_major() {
    remaining=$1
    result=
    carry=1

    while [ -n "$remaining" ]; do
        last=${remaining#${remaining%?}}
        remaining=${remaining%?}

        if [ "$carry" -eq 0 ]; then
            digit=$last
        else
            case "$last" in
                0) digit=1; carry=0 ;;
                1) digit=2; carry=0 ;;
                2) digit=3; carry=0 ;;
                3) digit=4; carry=0 ;;
                4) digit=5; carry=0 ;;
                5) digit=6; carry=0 ;;
                6) digit=7; carry=0 ;;
                7) digit=8; carry=0 ;;
                8) digit=9; carry=0 ;;
                9) digit=0 ;;
                *) fail "invalid major version" ;;
            esac
        fi

        result=$digit$result
    done

    [ "$carry" -eq 0 ] || result=1$result
    printf '%s\n' "$result"
}

next_version() {
    version=$1
    case "$version" in
        ''|*[!0-9.]*|.*|*.) fail "malformed version '$version'" ;;
    esac

    old_ifs=$IFS
    IFS=.
    set -- $version
    IFS=$old_ifs

    [ "$#" -eq 3 ] || fail "malformed version '$version'"
    major=$1
    minor=$2
    patch=$3

    case "$major" in
        0|[1-9]*) ;;
        *) fail "malformed version '$version'" ;;
    esac
    case "$minor" in
        [0-9]) ;;
        *) fail "malformed version '$version'" ;;
    esac
    case "$patch" in
        [0-9]) ;;
        *) fail "malformed version '$version'" ;;
    esac

    if [ "$patch" -lt 9 ]; then
        patch=$((patch + 1))
    else
        patch=0
        if [ "$minor" -lt 9 ]; then
            minor=$((minor + 1))
        else
            minor=0
            major=$(increment_major "$major")
        fi
    fi

    printf '%s.%s.%s\n' "$major" "$minor" "$patch"
}

bump_version() {
    version=$(read_version)
    bumped=$(next_version "$version")
    printf '%s\n' "$bumped" > "$ROOT/VERSION"
    git -C "$ROOT" add -- VERSION
}
