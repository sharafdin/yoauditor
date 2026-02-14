# Intentional issues: hardcoded secret, weak comparison, no rate limit

API_KEY = "sk-live-abc123secret456"  # Hardcoded secret

def check_password(user_input, stored_hash):
    # Timing attack: == short-circuits, not constant-time
    return user_input == stored_hash

def login(username, password):
    # No rate limiting - brute force possible
    user = get_user(username)
    if user and check_password(password, user.hash):
        return create_session(user)
    return None
