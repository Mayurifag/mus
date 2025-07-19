import pytest
import pytest_asyncio
import asyncio
from src.mus.util.redis_utils import set_app_write_lock, check_and_clear_app_write_lock


@pytest.mark.asyncio
async def test_set_app_write_lock(fake_redis):
    file_path = "/test/path/file.mp3"
    
    await set_app_write_lock(file_path)
    
    key = f"app_write_lock:{file_path}"
    result = await fake_redis.get(key)
    assert result == b"1"


@pytest.mark.asyncio
async def test_check_and_clear_app_write_lock_exists(fake_redis):
    file_path = "/test/path/file.mp3"
    key = f"app_write_lock:{file_path}"
    
    await fake_redis.set(key, "1")
    
    result = await check_and_clear_app_write_lock(file_path)
    
    assert result is True
    assert await fake_redis.get(key) is None


@pytest.mark.asyncio
async def test_check_and_clear_app_write_lock_not_exists(fake_redis):
    file_path = "/test/path/file.mp3"
    
    result = await check_and_clear_app_write_lock(file_path)
    
    assert result is False


@pytest.mark.asyncio
async def test_lock_expires_after_ttl(fake_redis):
    file_path = "/test/path/file.mp3"
    
    await set_app_write_lock(file_path)
    
    key = f"app_write_lock:{file_path}"
    ttl = await fake_redis.ttl(key)
    assert ttl > 0
    assert ttl <= 5


@pytest.mark.asyncio
async def test_multiple_locks_different_files(fake_redis):
    file_path1 = "/test/path/file1.mp3"
    file_path2 = "/test/path/file2.mp3"
    
    await set_app_write_lock(file_path1)
    await set_app_write_lock(file_path2)
    
    key1 = f"app_write_lock:{file_path1}"
    key2 = f"app_write_lock:{file_path2}"
    
    assert await fake_redis.get(key1) == b"1"
    assert await fake_redis.get(key2) == b"1"
    
    result1 = await check_and_clear_app_write_lock(file_path1)
    result2 = await check_and_clear_app_write_lock(file_path2)
    
    assert result1 is True
    assert result2 is True
    assert await fake_redis.get(key1) is None
    assert await fake_redis.get(key2) is None


@pytest.mark.asyncio
async def test_lock_overwrite_same_file(fake_redis):
    file_path = "/test/path/file.mp3"
    key = f"app_write_lock:{file_path}"
    
    await set_app_write_lock(file_path)
    first_ttl = await fake_redis.ttl(key)
    
    await asyncio.sleep(0.1)
    
    await set_app_write_lock(file_path)
    second_ttl = await fake_redis.ttl(key)
    
    assert second_ttl >= first_ttl
    assert await fake_redis.get(key) == b"1"


@pytest.mark.asyncio
async def test_intent_lock_pattern_workflow(fake_redis):
    file_path = "/test/music/track.mp3"
    
    await set_app_write_lock(file_path)
    
    lock_exists = await check_and_clear_app_write_lock(file_path)
    assert lock_exists is True
    
    lock_exists_again = await check_and_clear_app_write_lock(file_path)
    assert lock_exists_again is False
