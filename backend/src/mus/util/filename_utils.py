import re


def sanitize_filename(name: str) -> str:
    """
    Sanitize a string to be safe for use as a filename.
    Removes characters that are not allowed in filenames on most filesystems.
    """
    return re.sub(r'[<>:"/\\|?*]', "", name)


def generate_track_filename(artist: str, title: str, extension: str) -> str:
    """
    Generate a filename in the format "Artist1, Artist2 - Title.extension".
    Handles multiple artists by converting semicolon-separated format to comma-separated.

    Args:
        artist: Artist name(s), potentially semicolon-separated
        title: Track title
        extension: File extension (should include the dot, e.g., ".mp3")

    Returns:
        Sanitized filename in the format "Artist1, Artist2 - Title.extension"

    Raises:
        ValueError: If the resulting filename would be too long (>255 characters)
    """
    # Convert semicolon-separated artists to comma-separated for display
    formatted_artists = ", ".join(artist.split(";"))

    # Generate the filename
    filename = f"{sanitize_filename(formatted_artists)} - {sanitize_filename(title)}{extension}"

    # Check filename length
    if len(filename) > 255:
        raise ValueError("Filename too long")

    return filename
