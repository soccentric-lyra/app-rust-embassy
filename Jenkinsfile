// Jenkinsfile - multi-stage CI for the STM32 Rust (Embassy) application.
//
//   Checkout -> Toolchain -> Build -> Lint -> Format -> Image -> Package
//   -> Release (tags) -> Archive
//
// Package builds dist/<repo>-<version>.tar.gz (outputs, manifest.json,
// SHA256SUMS) on every run.  Release runs only for a tag v<version> and
// publishes dist/ as a GitHub release with the Jenkins secret-text credential
// github-release-token.  To release: bump the version, push, then push the
// tag (git tag v0.2.0 && git push origin v0.2.0).
//
// Builds run through scripts/docker-dev.sh (RCON-001). The Rust image is
// built on top of the shared stm32-dev base, so the Toolchain stage creates
// the base first when the agent does not have it yet.

pipeline {
    agent any

    options {
        skipDefaultCheckout()
        timestamps()
        timeout(time: 45, unit: 'MINUTES')
        buildDiscarder(logRotator(numToKeepStr: '30', artifactNumToKeepStr: '10'))
        disableConcurrentBuilds()
    }

    parameters {
        booleanParam(name: 'RUN_LINT', defaultValue: true, description: 'Run cargo clippy -D warnings')
        booleanParam(name: 'CHECK_FORMAT', defaultValue: true, description: 'Check cargo fmt (marks the build unstable on drift)')
    }

    environment {
        BASE_IMAGE  = 'stm32-dev:latest'
        STM32_IMAGE = 'stm32-dev-rust:latest'
    }

    stages {
        stage('Checkout') {
            steps {
                checkout scm
                sh '''
                    git log -1 --pretty=fuller
                    echo "commit: $(git rev-parse HEAD)"
                    echo "branch: $(git rev-parse --abbrev-ref HEAD)"
                '''
            }
        }

        stage('Toolchain') {
            steps {
                sh '''
                    docker version --format 'docker server: {{.Server.Version}}'
                    if ! docker image inspect "${BASE_IMAGE}" >/dev/null 2>&1; then
                        echo "Building ${BASE_IMAGE} from docker/Dockerfile.base"
                        docker build -t "${BASE_IMAGE}" -f docker/Dockerfile.base docker
                    fi
                    if ! docker image inspect "${STM32_IMAGE}" >/dev/null 2>&1; then
                        echo "Building ${STM32_IMAGE} from docker/Dockerfile"
                        make image
                    fi
                    docker image inspect --format 'base:  {{.Id}}' "${BASE_IMAGE}"
                    docker image inspect --format 'image: {{.Id}}' "${STM32_IMAGE}"
                    scripts/docker-dev.sh run rustc --version
                    scripts/docker-dev.sh run cargo --version
                '''
            }
        }

        stage('Build') {
            steps {
                sh 'make build'
            }
        }

        stage('Lint') {
            when { expression { return params.RUN_LINT } }
            steps {
                sh 'make lint'
            }
        }

        stage('Format') {
            when { expression { return params.CHECK_FORMAT } }
            steps {
                // rustfmt drift is reported, but does not block a firmware build.
                catchError(buildResult: 'UNSTABLE', stageResult: 'FAILURE') {
                    sh 'scripts/docker-dev.sh run env CARGO_HOME=/work/.cargo-home cargo fmt --all -- --check'
                }
            }
        }

        stage('Boot contract') {
            steps {
                sh 'python3 scripts/check_boot_contract.py'
            }
        }

        stage('Host tests') {
            steps {
                sh 'make test-host'
            }
        }

        stage('Image') {
            steps {
                sh '''
                    make size
                    python3 scripts/check_image.py build/embassy-app.img --version "$(ci/version.sh)"
                '''
            }
        }

        stage('Package') {
            steps {
                sh '''
                    python3 ci/package.py --name app-rust-embassy --version "$(ci/version.sh)" \\
                        build/embassy-app.elf build/embassy-app.bin build/embassy-app.hex build/embassy-app.img \\
                        README.md LICENSE
                '''
            }
        }

        stage('Release') {
            // Only for a tag build (Multibranch job with tag discovery).  The
            // tag must be v<version>; ci/github_release.py checks it.
            when { buildingTag() }
            steps {
                withCredentials([string(credentialsId: 'github-release-token', variable: 'GITHUB_TOKEN')]) {
                    sh 'python3 ci/github_release.py --tag "${TAG_NAME}" --version "$(ci/version.sh)" dist/*'
                }
            }
        }

        stage('Archive') {
            steps {
                archiveArtifacts artifacts: 'dist/*', fingerprint: true
            }
        }
    }

    post {
        always {
            deleteDir()
        }
        success {
            echo 'STM32 Rust application: build, clippy and image checks passed.'
        }
        failure {
            echo 'STM32 Rust application: pipeline failed - see the stage logs above.'
        }
    }
}
