<!-- This complete code (CallBack View) is a part of Layer8 Component -->
<script setup>
import { computed, ref } from "vue";
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'

const router = useRouter()
const code = ref(new URLSearchParams(window.location.search).get("code"))
// Cookie:     &http.Cookie{Name: "code", Value: code, Path: "/", MaxAge: 60 * 10, HttpOnly: true},
const token = ref(document.cookie.split(';').filter(item => item.trim().startsWith('token=')).map(c => c.split('=')[1])[0])
console.log("Auth JWT token: ", token.value)
const BACKEND_URL =  "http://localhost:6191"


onMounted(() => {
    setTimeout(() => {
        fetch(BACKEND_URL + "/api/login/layer8/auth", {
            method: "POST",
            headers: {
                "Content-Type": "Application/Json"
            },
            body: JSON.stringify({
                token: token.value
            })
        })
            .then(res => res.json())
            .then(data => {
                // Sleep for 30 seconds
                setTimeout(() => {
                    console.log("Sleeping for 30 seconds...")
                }, 30000);

                router.push({ name: 'profile' })
            })
            .catch(err => console.log(err))
    }, 1000);
})
</script>

<template>
    <div>
        <h1>Login with layer8...</h1>
    </div>
</template>

