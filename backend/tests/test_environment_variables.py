import os


def test_app_env_is_test():
    """Test that APP_ENV is set to 'test' during test execution."""
    app_env = os.getenv("APP_ENV")
    assert app_env == "test", f"APP_ENV should be 'test', but got '{app_env}'"
    assert (
        os.getenv("MUSIC_DIR") is None
    ), f"MUSIC_DIR should be None, but got '{os.getenv('MUSIC_DIR')}'"
