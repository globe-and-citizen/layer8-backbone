<script setup lang="ts">
import { ref } from 'vue';
import { getToken } from '@/utils.js';
import * as interceptorWasm from "layer8-interceptor-production";
import { getCurrentInstance } from 'vue';
const { appContext } = getCurrentInstance();
const backend_url = appContext.config.globalProperties.$backend_url;

const selectedFile = ref<File | null>(null);
const previewImage = ref<string | null>(null);
const errorMessage = ref<string | null>(null);
const successMessage = ref<string | null>(null);
const isLoading = ref(false);

const handleFileChange = (event: Event) => {
    const input = event.target as HTMLInputElement;
    if (input.files && input.files[0]) {
        selectedFile.value = input.files[0];
        
        // Create preview
        const reader = new FileReader();
        reader.onload = (e) => {
            previewImage.value = e.target?.result as string;
        };
        reader.readAsDataURL(input.files[0]);
        
        errorMessage.value = null;
    }
};

const uploadImage = async () => {
    if (!selectedFile.value) {
        errorMessage.value = 'Please select a file first';
        return;
    }

    const token = getToken('jwt');
    if (!token) {
        errorMessage.value = 'You need to be logged in to upload a profile picture';
        return;
    }

    try {
        isLoading.value = true;
        const payload = JSON.parse(atob(token.split('.')[1]));
        const username = payload.username;

        const formData = new FormData();
        formData.append('profile_pic', selectedFile.value);

        const response = await interceptorWasm.fetch(`${backend_url}/profile/${username}/upload`, {
            method: 'POST',
            body: formData
        });

        if (!response.ok) {
            const errorData = await response.json();
            throw new Error(errorData.error || 'Failed to upload image');
        }

        successMessage.value = 'Profile picture updated successfully!';
        // Redirect to profile after 2 seconds
        setTimeout(() => {
            window.location.href = '/profile';
        }, 2000);
    } catch (err) {
        errorMessage.value = err instanceof Error ? err.message : 'An unknown error occurred';
    } finally {
        isLoading.value = false;
    }
};
</script>

<template>
    <div class="upload-container">
        <div class="upload-card">
            <h1>Update Profile Picture</h1>
            
            <div v-if="previewImage" class="image-preview">
                <img :src="previewImage" alt="Preview of selected image" />
            </div>
            
            <div class="upload-form">
                <input 
                    type="file" 
                    id="profile-pic" 
                    accept="image/*" 
                    @change="handleFileChange"
                    class="file-input"
                />
                <label for="profile-pic" class="file-label">
                    Choose an image
                </label>
                
                <button 
                    @click="uploadImage" 
                    :disabled="!selectedFile || isLoading"
                    class="upload-button"
                >
                    <span v-if="isLoading">Uploading...</span>
                    <span v-else>Upload</span>
                </button>
                
                <div v-if="errorMessage" class="error-message">
                    {{ errorMessage }}
                </div>
                
                <div v-if="successMessage" class="success-message">
                    {{ successMessage }}
                </div>
            </div>
            
            <router-link to="/profile" class="back-link">
                Back to Profile
            </router-link>
        </div>
    </div>
</template>

<style scoped>
.upload-container {
    display: flex;
    justify-content: center;
    padding: 3rem 1rem;
    min-height: 100vh;
}

.upload-card {
    background: #fff;
    padding: 2rem;
    border-radius: 1rem;
    max-width: 500px;
    width: 100%;
    text-align: center;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
}

h1 {
    margin: 0 0 1.5rem;
    font-size: 1.8rem;
}

.image-preview {
    width: 200px;
    height: 200px;
    margin: 0 auto 1.5rem;
    border-radius: 50%;
    overflow: hidden;
    border: 2px solid #eee;
}

.image-preview img {
    width: 100%;
    height: 100%;
    object-fit: cover;
}

.upload-form {
    display: flex;
    flex-direction: column;
    gap: 1rem;
    margin-bottom: 1.5rem;
}

.file-input {
    display: none;
}

.file-label {
    display: inline-block;
    padding: 0.75rem 1.5rem;
    background-color: #f0f0f0;
    color: #333;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.3s;
}

.file-label:hover {
    background-color: #e0e0e0;
}

.upload-button {
    padding: 0.75rem 1.5rem;
    background-color: #4a6fa5;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.3s;
}

.upload-button:hover:not(:disabled) {
    background-color: #3a5a8f;
}

.upload-button:disabled {
    background-color: #cccccc;
    cursor: not-allowed;
}

.error-message {
    color: #d32f2f;
    margin-top: 1rem;
}

.success-message {
    color: #388e3c;
    margin-top: 1rem;
}

.back-link {
    display: inline-block;
    color: #4a6fa5;
    text-decoration: none;
    margin-top: 1rem;
}

.back-link:hover {
    text-decoration: underline;
}
</style>