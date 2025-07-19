import re


def sanitize_filename(name: str) -> str:
    return re.sub(r'[<>:"/\\|?*]', "", name)


def generate_track_filename(artist: str, title: str, extension: str) -> str:
    formatted_artists = ", ".join(artist.split(";"))
    filename = f"{sanitize_filename(formatted_artists)} - {sanitize_filename(title)}{extension}"

    if len(filename) > 255:
        raise ValueError("Filename too long")

    return filename
