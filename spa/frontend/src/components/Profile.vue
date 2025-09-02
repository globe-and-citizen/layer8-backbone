<script setup lang="ts">
import { onMounted, ref, watch, computed } from 'vue';
import { getToken } from '@/utils.js';
import router from '@/router';
import * as interceptorWasm from "layer8-interceptor-production";
import { getCurrentInstance } from 'vue';

const instance = getCurrentInstance();
if (!instance) {
    throw new Error('getCurrentInstance() returned null. This component must be used within a Vue component context.');
}
const { appContext } = instance;
const backend_url = appContext.config.globalProperties.$backend_url;

const profile = ref({
    username: "",
    metadata: {
        email_verified: false,
        bio: "",
        display_name: "",
        color: "",

    },
    profilePicture: ""
});

const showAuthModal = ref(false);
const authOptions = ref({
    email_verified: false,
    bio: false,
    display_name: false,
    color: false
});

const downloadProfilePicture = async () => {
    if (!profile.value.profilePicture) return;
    
    let image = profile.value.profilePicture.split('uploads/')[1];
    
    try {
        const response = await interceptorWasm.fetch(`${backend_url}/uploads/${image}`);
        if (!response.ok) {
            throw new Error('Network response was not ok');
        }
        
        const blob = await response.blob();
        
        // Create a temporary URL for the blob
        const url = window.URL.createObjectURL(blob);
        
        // Create a temporary anchor element to trigger download
        const a = document.createElement('a');
        a.href = url;
        a.download = `${profile.value.username}_profile_picture.${image.split('.').pop()}`;
        document.body.appendChild(a);
        a.click();
        
        // Clean up
        document.body.removeChild(a);
        window.URL.revokeObjectURL(url);
    } catch (error) {
        console.error('Error downloading profile picture:', error);
    }
};

const openAuthModal = () => {
    showAuthModal.value = true;
};

const loadProfilePicture = async (pictureUrl: string) => {
    let image = pictureUrl.split('uploads/')[1];
    let new_url = "";
    await interceptorWasm.fetch(`${backend_url}/uploads/${image}`)
        .then(response => {
            if (!response.ok) {
                throw new Error('Network response was not ok');
            }
            return response.blob();
        })
        .then(blob => {
            new_url = window.URL.createObjectURL(blob);
        })
        .catch(error => {
            console.error('Error loading profile picture:', error);
        });

    return new_url;
}

const closeAuthModal = () => {
    showAuthModal.value = false;
};

const initializeAuth = async () => {
    const token = getToken('jwt');
    if (!token) {
        console.error('No token found');
        return;
    }

    try {
        const response = await interceptorWasm.fetch(`${backend_url}/update-user-profile-metadata`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'Authorization': `Bearer ${token}`
            },
            body: JSON.stringify(authOptions.value)
        });

        if (response.ok) {
            // Refresh the profile data
            const payload = JSON.parse(atob(token.split('.')[1]));
            const username = payload.username;

            const profileResponse = await interceptorWasm.fetch(`${backend_url}/profile/${username}`);
            const data = await profileResponse.json();

            profile.value.username = username;
            profile.value.profilePicture = data.profilePicture || "";

            console.log("***************Profile metadata***************: ", data.metadata)

            if (data.metadata) {
                profile.value.metadata = {
                    email_verified: data.metadata.email_verified || false,
                    bio: data.metadata.bio || "",
                    display_name: data.metadata.display_name || "",
                    color: data.metadata.color || "",
                };
            }

            closeAuthModal();
        } else {
            console.error('Failed to initialize auth');
        }
    } catch (err) {
        console.error('Error initializing auth:', err);
    }
};

const loginWithLayer8Popup = async () => {
    const response = await interceptorWasm.fetch(`${backend_url}/api/login/layer8/auth`)
    const data = await response.json()
    // create opener window
    const popup = window.open(data.authURL, "Login with Layer8", "width=1200,height=900");
    const token = getToken('jwt');
    if (!token) {
        console.error('No token found');
        return;
    }
    window.addEventListener("message", async (event) => {
        if (event.data.redr) {
            setTimeout(() => {
                interceptorWasm.fetch(`${backend_url}/authorization-callback`, {
                    method: "POST",
                    headers: {
                        "Content-Type": "Application/Json",
                        'Authorization': `${token}`
                    },
                    body: JSON.stringify({
                        code: event.data.code
                    })
                })
                    .then(res => res.json())
                    .then(data => {
                        router.push({ name: 'profile' })
                        if (popup) {
                            popup.close();
                        }
                        // Refetch the profile data
                        fetchProfileData();
                    })
                    .catch(err => console.log(err))
            }, 1000);
        }
    });
}

// Compute reputation score based on filled metadata
const reputationScore = computed(() => {
    if (!profile.value.metadata) return 0;

    const metadata = profile.value.metadata;
    let score = 0;

    // Count each filled metadata (email_verified counts if true)
    if (metadata.email_verified) score++;
    if (metadata.bio) score++;
    if (metadata.display_name) score++;
    if (metadata.color) score++;

    return score;
});

// Compute reputation color based on score
const reputationColor = computed(() => {
    const score = reputationScore.value;
    if (score <= 1) return '#e74c3c'; // red
    if (score === 2) return '#f39c12'; // orange/yellow
    if (score === 3 || score === 4) return '#3498db'; // blue
    return '#2ecc71'; // green (for 5)
});

const fetchProfileData = async () => {
    const token = getToken('jwt');
    if (!token) {
        console.error('No token found');
        return;
    }

    const payload = JSON.parse(atob(token.split('.')[1]));
    const username = payload.username;

    try {
        const response = await interceptorWasm.fetch(`${backend_url}/profile/${username}`);
        const data = await response.json();

        profile.value.username = username;
        profile.value.profilePicture = data.profilePicture || "";

        if (data.metadata) {
            profile.value.metadata = {
                email_verified: data.metadata.email_verified || false,
                bio: data.metadata.bio || "",
                display_name: data.metadata.display_name || "",
                color: data.metadata.color || "",
            };
        }
    } catch (err) {
        console.error('Error fetching profile:', err);
    }
};

onMounted(() => {
    fetchProfileData();
});

let profilePictureUrl = ref("");
watch(() => profile.value.profilePicture, async (newUrl) => {
    if (newUrl) {
        profilePictureUrl.value = await loadProfilePicture(newUrl);
    } else {
        profilePictureUrl.value = "";
    }
}, { immediate: true });

</script>

<template>
    <div class="profile-container">
        <div class="profile-card">
            <div class="profile-content">
                <!-- Left side - Profile info -->
                <div class="profile-info">
                    <!-- Profile picture section -->
                    <div v-if="profilePictureUrl" class="profile-picture">
                        <img :src="profilePictureUrl" :alt="`${profile.username}'s profile picture`" />
                    </div>
                    <div v-else class="profile-picture placeholder">
                        <span>{{ profile.username.charAt(0).toUpperCase() }}</span>
                    </div>

                    <h1>{{ profile.username }}</h1>

                    <div class="metadata-grid">
                        <div class="metadata-item">
                            <strong>Email Verified:</strong>
                            <span :class="profile.metadata.email_verified ? 'verified' : 'not-verified'">
                                {{ profile.metadata.email_verified ? 'Verified' : 'Not Verified' }}
                            </span>
                        </div>

                        <div v-if="profile.metadata.bio" class="metadata-item">
                            <strong>Bio:</strong>
                            <span>{{ profile.metadata.bio }}</span>
                        </div>

                        <div v-if="profile.metadata.display_name" class="metadata-item">
                            <strong>Display Name:</strong>
                            <span>{{ profile.metadata.display_name }}</span>
                        </div>

                        <div v-if="profile.metadata.color" class="metadata-item address">
                            <strong>Color:</strong>
                            <span>{{ profile.metadata.color }}</span>
                        </div>
                    </div>

                    <button @click="downloadProfilePicture" class="download-button" :disabled="!profile.profilePicture">
                        Download Profile Picture
                    </button>
                    <button @click="loginWithLayer8Popup" class="init-auth-button">
                        Initialize Auth
                    </button>
                </div>

                <!-- Right side - Reputation -->
                <div class="reputation-section">
                    <h2>Profile Completeness</h2>
                    <div class="reputation-score">
                        <span class="score">{{ reputationScore }}/5</span>
                        <div class="score-bar">
                            <div class="score-fill"
                                :style="{ width: `${(reputationScore / 5) * 100}%`, backgroundColor: reputationColor }">
                            </div>
                        </div>
                    </div>

                    <div class="reputation-details">
                        <h3>Details:</h3>
                        <ul>
                            <li :class="{ 'completed': profile.metadata.email_verified }">
                                Email Verified
                            </li>
                            <li :class="{ 'completed': profile.metadata.bio }">
                                Bio Provided
                            </li>
                            <li :class="{ 'completed': profile.metadata.display_name }">
                                Display Name Provided
                            </li>
                            <li :class="{ 'completed': profile.metadata.color }">
                                Color Provided
                            </li>
                        </ul>
                    </div>

                    <div class="reputation-benefits">
                        <h3>Benefits:</h3>
                        <p v-if="reputationScore >= 3">
                            <i class="fas fa-check-circle"></i> Your profile is considered trustworthy
                        </p>
                        <p v-else>
                            <i class="fas fa-info-circle"></i> Complete more details to increase trust
                        </p>
                    </div>
                </div>
                <div v-if="showAuthModal" class="modal-overlay">
                    <div class="auth-modal">
                        <div class="modal-header">
                            <h2>Initialize Authentication</h2>
                            <button @click="closeAuthModal" class="close-button">&times;</button>
                        </div>
                        <div class="modal-body">
                            <p>Select which information you want to initialize:</p>

                            <div class="auth-option">
                                <input type="checkbox" id="email_verified" v-model="authOptions.email_verified">
                                <label for="email_verified">Share Email Verification Details</label>
                            </div>

                            <div class="auth-option">
                                <input type="checkbox" id="bio" v-model="authOptions.bio">
                                <label for="bio">Share Bio</label>
                            </div>

                            <div class="auth-option">
                                <input type="checkbox" id="display_name" v-model="authOptions.display_name">
                                <label for="display_name">Share Display Name</label>
                            </div>

                            <div class="auth-option">
                                <input type="checkbox" id="color" v-model="authOptions.color">
                                <label for="color">Share Color</label>
                            </div>
                        </div>
                        <div class="modal-footer">
                            <button @click="closeAuthModal" class="cancel-button">Cancel</button>
                            <button @click="initializeAuth" class="confirm-button">Initialize</button>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.profile-container {
    display: flex;
    justify-content: center;
    padding: 3rem 1rem;
    min-height: 100vh;
    max-width: 1200px;
    /* Wider container */
    margin: 0 auto;
}

.init-auth-button {
    display: block;
    margin: 0 auto 1.5rem;
    padding: 0.7rem 1.5rem;
    background-color: #4a6fa5;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 1rem;
    transition: all 0.3s;
}

.init-auth-button:hover {
    background-color: #3a5a8f;
    transform: translateY(-2px);
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
}

.modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1000;
}

.auth-modal {
    background: white;
    border-radius: 8px;
    width: 90%;
    max-width: 500px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
    overflow: hidden;
}

.modal-header {
    padding: 1.5rem;
    border-bottom: 1px solid #eee;
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.modal-header h2 {
    margin: 0;
    color: #2c3e50;
    text-align: left;
}

.close-button {
    background: none;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    color: #95a5a6;
}

.modal-body {
    padding: 1.5rem;
}

.modal-body p {
    margin-bottom: 1.5rem;
    color: #2c3e50;
}

.auth-option {
    margin-bottom: 1rem;
    display: flex;
    align-items: center;
}

.auth-option input {
    margin-right: 0.75rem;
}

.auth-option label {
    cursor: pointer;
}

.modal-footer {
    padding: 1.5rem;
    border-top: 1px solid #eee;
    display: flex;
    justify-content: flex-end;
    gap: 1rem;
}

.cancel-button {
    padding: 0.7rem 1.5rem;
    background: #f8f9fa;
    border: 1px solid #ddd;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.3s;
}

.cancel-button:hover {
    background: #e9ecef;
}

.confirm-button {
    padding: 0.7rem 1.5rem;
    background: #4a6fa5;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.3s;
}

.confirm-button:hover {
    background: #3a5a8f;
}

.profile-card {
    background: #fff;
    padding: 2.5rem;
    border-radius: 1rem;
    width: 100%;
    text-align: center;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
    border: 2px solid #b9b9b9;
}

.profile-content {
    display: flex;
    gap: 3rem;
    text-align: left;
}

.profile-info {
    flex: 2;
}

.reputation-section {
    flex: 1;
    padding: 1.5rem;
    background: #f8f9fa;
    border-radius: 8px;
    border-left: 1px solid #e0e0e0;
    min-width: 280px;
}

.profile-picture {
    width: 140px;
    height: 140px;
    margin: 0 auto 1.5rem;
    border-radius: 50%;
    overflow: hidden;
    position: relative;
}

.profile-picture img {
    width: 100%;
    height: 100%;
    object-fit: cover;
}

.profile-picture.placeholder {
    background-color: #4a6fa5;
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
    font-size: 3.5rem;
    font-weight: bold;
}

h1 {
    margin: 0.5rem 0 1rem;
    font-size: 2rem;
    color: #2c3e50;
    text-align: center;
}

h2 {
    color: #2c3e50;
    margin-bottom: 1.5rem;
    text-align: center;
}

h3 {
    color: #4a6fa5;
    margin: 1.5rem 0 0.5rem;
    font-size: 1.1rem;
}

.metadata-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 1.2rem;
    margin: 2rem 0;
}

.metadata-item {
    padding: 0.8rem;
    background: #f8f9fa;
    border-radius: 8px;
}

.metadata-item strong {
    display: block;
    margin-bottom: 0.3rem;
    color: #000000;
    /* Make it bold */
    font-weight: bold;
}

.metadata-item.address {
    grid-column: 1 / -1;
}

.verified {
    color: #27ae60;
    font-weight: 500;
}

.not-verified {
    color: #e74c3c;
    font-weight: 500;
}

.download-button {
    display: inline-block;
    margin-top: 1.5rem;
    padding: 0.7rem 1.5rem;
    background-color: #4a6fa5;
    color: white;
    text-decoration: none;
    border-radius: 6px;
    border: none;
    cursor: pointer;
    transition: all 0.3s;
    font-size: 1rem;
}

.download-button:hover {
    background-color: #3a5a8f;
    transform: translateY(-2px);
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
}

.download-button:disabled {
    background-color: #cccccc;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
}

/* Reputation styles */
.reputation-score {
    margin: 1.5rem 0;
    text-align: center;
}

.score {
    font-size: 1.5rem;
    font-weight: bold;
    color: #2c3e50;
    display: block;
    margin-bottom: 0.5rem;
}

.score-bar {
    height: 12px;
    background: #e0e0e0;
    border-radius: 6px;
    overflow: hidden;
    margin-bottom: 1rem;
}

.score-fill {
    height: 100%;
    transition: all 0.3s ease;
}

.reputation-details ul {
    list-style: none;
    padding: 0;
    margin: 0;
}

.reputation-details li {
    padding: 0.5rem 0;
    position: relative;
    padding-left: 1.5rem;
}

.reputation-details li:before {
    content: "•";
    position: absolute;
    left: 0;
    color: #95a5a6;
}

.reputation-details li.completed {
    color: #27ae60;
}

.reputation-details li.completed:before {
    content: "✓";
    color: #27ae60;
}

.reputation-benefits p {
    padding: 0.5rem;
    background: #e8f4fd;
    border-radius: 6px;
    color: #2980b9;
}

.reputation-benefits i {
    margin-right: 0.5rem;
}

@media (max-width: 900px) {
    .profile-content {
        flex-direction: column;
    }

    .reputation-section {
        border-left: none;
        border-top: 1px solid #e0e0e0;
    }
}

.init-auth-button {
    display: block;
    /* margin: 0 auto 1.5rem; */
    margin-top: 10px;
    margin-left: 40px;
    padding: 0.7rem 1.5rem;
    /* Add a space on top */
    background-color: #4a6fa5;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 1rem;
    transition: all 0.3s;
}

.init-auth-button:hover {
    background-color: #3a5a8f;
    transform: translateY(-2px);
    box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
}

.modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1000;
}

.auth-modal {
    background: white;
    border-radius: 8px;
    width: 90%;
    max-width: 500px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
    overflow: hidden;
}

.modal-header {
    padding: 1.5rem;
    border-bottom: 1px solid #eee;
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.modal-header h2 {
    margin: 0;
    color: #2c3e50;
    text-align: left;
}

.close-button {
    background: none;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    color: #95a5a6;
}

.modal-body {
    padding: 1.5rem;
}

.modal-body p {
    margin-bottom: 1.5rem;
    color: #2c3e50;
}

.auth-option {
    margin-bottom: 1rem;
    display: flex;
    align-items: center;
}

.auth-option input {
    margin-right: 0.75rem;
}

.auth-option label {
    cursor: pointer;
}

.modal-footer {
    padding: 1.5rem;
    border-top: 1px solid #eee;
    display: flex;
    justify-content: flex-end;
    gap: 1rem;
}

.cancel-button {
    padding: 0.7rem 1.5rem;
    background: #f8f9fa;
    border: 1px solid #ddd;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.3s;
}

.cancel-button:hover {
    background: #e9ecef;
}

.confirm-button {
    padding: 0.7rem 1.5rem;
    background: #4a6fa5;
    color: white;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.3s;
}

.confirm-button:hover {
    background: #3a5a8f;
}
</style>