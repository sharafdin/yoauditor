API_KEY = "sk-live-abc123secret456"

def check_password(user_input, stored_hash):
    return user_input == stored_hash

def login(username, password):
    user = get_user(username)
    if user and check_password(password, user.hash):
        return create_session(user)
    return None
