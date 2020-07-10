#ifndef DISNEY_H
#define DISNEY_H

#include "utils.glsl"
#include "structs.glsl"

#define BSDF_TYPE_REFLECTED 0
#define BSDF_TYPE_TRANSMITTED 1
#define BSDF_TYPE_SPECULAR 2

float sqr(const float x) { return x * x; }

bool Refract(const vec3 wi, const vec3 n, const float eta, inout vec3 wt)
{
    const float cosThetaI = dot(n, wi);
    const float sin2ThetaI = max(0.0f, 1.0f - cosThetaI * cosThetaI);
    const float sin2ThetaT = eta * eta * sin2ThetaI;
    if (sin2ThetaT >= 1) {
        return false;// TIR
    }

    float cosThetaT = sqrt(1.0f - sin2ThetaT);
    wt = eta * (wi * -1.0f) + (eta * cosThetaI - cosThetaT) * vec3(n);
    return true;
}

float SchlickFresnel(const float u)
{
    const float m = clamp(1 - u, 0.0f, 1.0f);
    return float(m * m) * (m * m) * m;
}

void mix_spectra(const vec3 a, const vec3 b, const float t, inout vec3 result) { result = (1.0f - t) * a + t * b; }
void mix_one_with_spectra(const vec3 b, const float t, inout vec3 result) { result = (1.0f - t) + t * b; }
void mix_spectra_with_one(const vec3 a, const float t, inout vec3 result) { result = (1.0f - t) * a + t; }
float microfacet_alpha_from_roughness(const float roughness) { return max(0.001f, roughness * roughness); }
void microfacet_alpha_from_roughness(const float roughness, const float anisotropy, inout float alpha_x, inout float alpha_y)
{
    const float square_roughness = roughness * roughness;
    const float aspect = sqrt(1.0f + anisotropy * (anisotropy < 0 ? 0.9f : -0.9f));
    alpha_x = max(0.001f, square_roughness / aspect);
    alpha_y = max(0.001f, square_roughness * aspect);
}

float GTR1(const float NDotH, const float a)
{
    if (a >= 1)
    return INVPI;
    const float a2 = a * a;
    const float t = 1 + (a2 - 1) * NDotH * NDotH;
    return (a2 - 1) / (PI * log(a2) * t);
}

float GTR2(const float NDotH, const float a)
{
    const float a2 = a * a;
    const float t = 1.0f + (a2 - 1.0f) * NDotH * NDotH;
    return a2 / (PI * t * t);
}

float SmithGGX(const float NDotv, const float alphaG)
{
    const float a = alphaG * alphaG;
    const float b = NDotv * NDotv;
    return 1.0f / (NDotv + sqrt(a + b - a * b));
}

float Fr(const float VDotN, const float eio)
{
    const float SinThetaT2 = sqr(eio) * (1.0f - VDotN * VDotN);
    if (SinThetaT2 > 1.0f)
    return 1.0f;// TIR
    const float LDotN = sqrt(1.0f - SinThetaT2);
    const float eta = 1.0f / eio;
    const float r1 = (VDotN - eta * LDotN) / (VDotN + eta * LDotN);
    const float r2 = (LDotN - eta * VDotN) / (LDotN + eta * VDotN);
    return 0.5f * (sqr(r1) + sqr(r2));
}

vec3 SafeNormalize(const vec3 a)
{
    const float ls = dot(a, a);
    if (ls > 0.0f)
    return a * (1.0f / sqrt(ls));
    else
    return vec3(0);
}

float BSDFPdf(const ShadingData shadingData, const vec3 N, const vec3 wo, const vec3 wi)
{
    float bsdfPdf = 0.0f, brdfPdf;
    if (dot(wi, N) <= 0.0f)
    brdfPdf = INV2PI * shadingData.subsurface * 0.5f;
    else
    {
        const float F = Fr(dot(N, wo), shadingData.eta);
        const vec3 halfway = SafeNormalize(wi + wo);
        const float cosThetaHalf = abs(dot(halfway, N));
        const float pdfHalf = GTR2(cosThetaHalf, shadingData.roughness) * cosThetaHalf;
        // calculate pdf for each method given outgoing light vector
        const float pdfSpec = 0.25f * pdfHalf / max(1.e-6f, dot(wi, halfway));
        const float pdfDiff = abs(dot(wi, N)) * INVPI * (1.0f - shadingData.subsurface);
        bsdfPdf = pdfSpec * F;
        brdfPdf = mix(pdfDiff, pdfSpec, 0.5f);
    }
    return mix(brdfPdf, bsdfPdf, shadingData.transmission);
}

// evaluate the BSDF for a given pair of directions
vec3 BSDFEval(const ShadingData shadingData, const vec3 N, const vec3 wo, const vec3 wi, const float t, const bool backfacing)
{
    const float NDotL = dot(N, wi);
    const float NDotV = dot(N, wo);
    const vec3 H = normalize(wi + wo);
    const float NDotH = dot(N, H);
    const float LDotH = dot(wi, H);
    const vec3 Cdlin = shadingData.color.xyz;
    const float Cdlum = .3f * Cdlin.x + .6f * Cdlin.y + .1f * Cdlin.z;// luminance approx.
    const vec3 Ctint = Cdlum > 0.0f ? Cdlin / Cdlum : vec3(1.0f);// normalize lum. to isolate hue+sat
    const vec3 Cspec0 = mix(shadingData.specular * .08f * mix(vec3(1.0f), Ctint, shadingData.specular_tint), Cdlin, shadingData.metallic);
    vec3 bsdf = vec3(0);
    vec3 brdf = vec3(0);
    if (shadingData.transmission > 0.0f)
    {
        // evaluate BSDF
        if (NDotL <= 0)
        {
            // transmission Fresnel
            const float F = Fr(NDotV, shadingData.eta);
            bsdf = vec3((1.0f - F) / abs(NDotL) * (1.0f - shadingData.metallic) * shadingData.transmission);
        }
        else
        {
            // specular lobe
            const float a = shadingData.roughness;
            const float Ds = GTR2(NDotH, a);

            // Fresnel term with the microfacet normal
            const float FH = Fr(LDotH, shadingData.eta);
            const vec3 Fs = mix(Cspec0, vec3(1.0f), FH);
            const float Gs = SmithGGX(NDotV, a) * SmithGGX(NDotL, a);
            bsdf = (Gs * Ds) * Fs;
        }
    }
    if (shadingData.transmission < 1.0f)
    {
        // evaluate BRDF
        if (NDotL <= 0)
        {
            if (shadingData.subsurface > 0.0f)
            {
                // take sqrt to account for entry/exit of the ray through the medium
                // this ensures transmitted light corresponds to the diffuse model
                const vec3 s = vec3(sqrt(shadingData.color.x), sqrt(shadingData.color.y), sqrt(shadingData.color.z));
                const float FL = SchlickFresnel(abs(NDotL)), FV = SchlickFresnel(NDotV);
                const float Fd = (1.0f - 0.5f * FL) * (1.0f - 0.5f * FV);
                brdf = INVPI * s * shadingData.subsurface * Fd * (1.0f - shadingData.metallic);
            }
        }
        else
        {
            // specular
            const float a = shadingData.roughness;
            const float Ds = GTR2(NDotH, a);

            // Fresnel term with the microfacet normal
            const float FH = SchlickFresnel(LDotH);
            const vec3 Fs = mix(Cspec0, vec3(1), FH);
            const float Gs = SmithGGX(NDotV, a) * SmithGGX(NDotL, a);

            // Diffuse fresnel - go from 1 at normal incidence to .5 at grazing
            // and mix in diffuse retro-reflection based on roughness
            const float FL = SchlickFresnel(NDotL), FV = SchlickFresnel(NDotV);
            const float Fd90 = 0.5 + 2.0f * LDotH * LDotH * a;
            const float Fd = mix(1.0f, Fd90, FL) * mix(1.0f, Fd90, FV);

            // clearcoat (ior = 1.5 -> F0 = 0.04)
            const float Dr = GTR1(NDotH, mix(.1, .001, shadingData.clearcoat_gloss));
            const float Fc = mix(.04f, 1.0f, FH);
            const float Gr = SmithGGX(NDotL, .25) * SmithGGX(NDotV, .25);

            brdf = INVPI * Fd * Cdlin * (1.0f - shadingData.metallic) * (1.0f - shadingData.subsurface) + Gs * Fs * Ds + shadingData.clearcoat * Gr * Fc * Dr;
        }
    }

    const vec3 final = mix(brdf, bsdf, shadingData.transmission);

    if (backfacing) {
        return final * exp(-vec3(shadingData.absorption) * t);
    }
    else {
        return final;
    }
}

// generate an importance sampled BSDF direction
void BSDFSample(const ShadingData shadingData, const vec3 T, const vec3 B, const vec3 N,
                const vec3 wo, inout vec3 wi, inout float pdf, inout int type,
                const float t, const bool backfacing, const float r3, const float r4)
{
    if (r3 < shadingData.transmission)
    {
        // sample BSDF
        float F = Fr(dot(N, wo), shadingData.eta);
        if (r4 < F)// sample reflectance or transmission based on Fresnel term
        {
            // sample reflection
            const float r1 = r3 / shadingData.transmission;
            const float r2 = r4 / F;
            const float cosThetaHalf = sqrt((1.0f - r2) / (1.0f + (sqr(shadingData.roughness) - 1.0f) * r2));
            const float sinThetaHalf = sqrt(max(0.0f, 1.0f - sqr(cosThetaHalf)));
            const float sinPhiHalf = sin(r1 * TWOPI);
            const float cosPhiHalf = cos(r1 * TWOPI);
            vec3 halfway = T * (sinThetaHalf * cosPhiHalf) + B * (sinThetaHalf * sinPhiHalf) + N * cosThetaHalf;
            if (dot(halfway, wo) <= 0.0f)
            halfway *= -1.0f;// ensure half angle in same hemisphere as wo
            type = BSDF_TYPE_REFLECTED;
            wi = reflect(wo * -1.0f, halfway);
        }
        else // sample transmission
        {
            pdf = 0;
            if (Refract(wo, N, shadingData.eta, wi))
            type = BSDF_TYPE_SPECULAR, pdf = (1.0f - F) * shadingData.transmission;
            return;
        }
    }
    else // sample BRDF
    {
        const float r1 = (r3 - shadingData.transmission) / (1 - shadingData.transmission);
        if (r4 < 0.5f)
        {
            // sample diffuse
            const float r2 = r4 * 2;
            vec3 d;
            if (r2 < shadingData.subsurface)
            {
                const float r5 = r2 / shadingData.subsurface;
                d = DiffuseReflectionUniform(r1, r5);
                type = BSDF_TYPE_TRANSMITTED, d.z *= -1.0f;
            }
            else
            {
                const float r5 = (r2 - shadingData.subsurface) / (1 - shadingData.subsurface);
                d = DiffuseReflectionCosWeighted(r1, r5);
                type = BSDF_TYPE_REFLECTED;
            }
            wi = T * d.x + B * d.y + N * d.z;
        }
        else
        {
            // sample specular
            const float r2 = (r4 - 0.5f) * 2.0f;
            const float cosThetaHalf = sqrt((1.0f - r2) / (1.0f + (sqr(shadingData.roughness) - 1.0f) * r2));
            const float sinThetaHalf = sqrt(max(0.0f, 1.0f - sqr(cosThetaHalf)));
            const float sinPhiHalf = sin(r1 * TWOPI);
            const float cosPhiHalf = cos(r1 * TWOPI);
            vec3 halfway = T * (sinThetaHalf * cosPhiHalf) + B * (sinThetaHalf * sinPhiHalf) + N * cosThetaHalf;
            if (dot(halfway, wo) <= 0.0f)
            halfway *= -1.0f;// ensure half angle in same hemisphere as wi
            wi = reflect(wo * -1.0f, halfway);
            type = BSDF_TYPE_REFLECTED;
        }
    }
    pdf = BSDFPdf(shadingData, N, wo, wi);
}

vec3 EvaluateBSDF(const ShadingData shadingData, const vec3 iN, const vec3 T, const vec3 B, const vec3 wo, const vec3 wi, inout float pdf)
{
    const vec3 bsdf = BSDFEval(shadingData, iN, wo, wi, 0.0f, false);
    pdf = BSDFPdf(shadingData, iN, wo, wi);
    return bsdf;
}

vec3 SampleBSDF(const ShadingData shadingData, vec3 iN, const vec3 N,
                const vec3 T, const vec3 B, const vec3 wo, const float t,
                const bool backfacing, const float r3, const float r4,
                inout vec3 wi, inout float pdf, inout bool specular)
{
    int type;
    BSDFSample(shadingData, T, B, N, wo, wi, pdf, type, t, backfacing, r3, r4);
    specular = type != BSDF_TYPE_REFLECTED;

    return BSDFEval(shadingData, iN, wo, wi, t, backfacing);
}
    #endif