# rhwp Desktop

`rhwp Desktop`는 [edwardkim/rhwp](https://github.com/edwardkim/rhwp)를 기반으로 만든 고성능 HWP/HWPX 데스크톱 뷰어 및 에디터입니다.

이 프로젝트는 웹 기반의 `rhwp` 엔진을 활용해, 우분투와 윈도우 환경에서 파일 더블 클릭만으로 문서를 바로 열고 읽고 편집할 수 있는 네이티브 경험을 제공하는 것을 목표로 제작되었습니다. 비영리 오픈소스 프로젝트이며, 현재 이 저장소는 Ubuntu용 `.deb`와 Windows용 설치 패키지(NSIS `.exe`, MSI)를 함께 관리합니다.

## 출처 및 라이선스

- 이 프로젝트는 `edwardkim/rhwp`의 소스 코드를 기반으로 합니다.
- 원작자의 MIT License를 준수하며, 라이선스 전문은 본 리포지토리의 [LICENSE](LICENSE) 파일에서 확인할 수 있습니다.

## 주요 특징

- Rust/WASM 기반 엔진으로 HWP/HWPX 문서를 빠르게 열고 렌더링합니다.
- 우분투용 `.deb` 설치 파일과 Windows용 설치 파일(NSIS `.exe`, MSI) 패키징 경로를 제공합니다.
- 시스템 파일 연결(MIME Type)로 `.hwp`, `.hwpx` 더블 클릭 실행을 지원합니다.
- 데스크톱 UI에서 문서를 열고 편집하고 다시 저장할 수 있습니다.
- 원본 `rhwp` 엔진을 바탕으로 데스크톱 배포와 파일 연결 경험에 초점을 맞췄습니다.

## 제작 목적

브라우저나 개발 도구 안에서만 동작하던 `rhwp` 엔진을 데스크톱 애플리케이션으로 감싸서, 일반 사용자가 문서 파일을 더블 클릭해 곧바로 읽고 편집할 수 있는 환경을 만드는 것이 이 프로젝트의 핵심 목적입니다.

특히 다음 경험을 목표로 합니다.

- Ubuntu 데스크톱에서 설치 후 바로 실행 가능한 HWP/HWPX 앱
- 파일 관리자에서 `.hwp`, `.hwpx`를 더블 클릭해 곧바로 문서 열기
- 수정 후 저장한 파일을 다시 읽고 이어서 편집할 수 있는 실사용 흐름

## 설치 방법

현재 공개 릴리즈는 Ubuntu `.deb`를 기준으로 배포되고, Windows 설치 파일은 이 저장소의 패키징/CI 경로를 통해 생성되도록 구성되어 있습니다.

### Ubuntu

1. GitHub Releases에서 `rhwp_0.1.0_amd64.deb` 파일을 받습니다.
2. 우분투 터미널에서 아래 명령으로 설치합니다.

```bash
cd ~/Downloads
sudo apt install ./rhwp_0.1.0_amd64.deb
```

설치 후에는 앱 목록에서 `rhwp`를 실행할 수 있고, 시스템 파일 연결이 적용되면 `.hwp`, `.hwpx` 파일을 더블 클릭해서 열 수 있습니다.

### Windows

1. GitHub Releases에 Windows 설치 파일(`.exe` 또는 `.msi`)이 게시된 경우 이를 받거나, 저장소의 Windows 패키징 워크플로로 생성한 설치 파일을 준비합니다.
2. 설치 프로그램을 실행해 `rhwp`를 설치합니다.
3. 설치 후 Windows `Default Apps`에서 `.hwp`, `.hwpx` 기본 앱으로 `rhwp`를 한 번 선택합니다.

Windows는 운영체제 정책상 앱이 기본 앱을 강제로 바꾸지 않으므로, Explorer 더블 클릭 열기를 완성하려면 위 기본 앱 선택 단계가 한 번 필요합니다.

## 현재 배포 상태

- Ubuntu `.deb` 설치 패키지: 제공 및 설치 검증 완료
- Windows NSIS `.exe` / MSI 패키지: 패키징, 파일 연결 등록, CI 릴리즈 경로 추가
- 더블 클릭 열기: Ubuntu는 직접 검증 완료, Windows는 기본 앱 선택 흐름을 전제로 지원

## 프로젝트 구성

- `rhwp-desktop`: Tauri 기반 데스크톱 앱 패키징
- `rhwp-studio`: 편집 UI와 문서 입출력 계층
- 루트 Rust 코드: HWP/HWPX 파싱, 렌더링, 저장 엔진

## 참고

원작 프로젝트와 엔진 자체에 대한 자세한 개발 정보는 `edwardkim/rhwp`와 이 저장소의 소스 코드를 참고하면 됩니다. 이 README는 사용자 입장에서 설치와 실행 흐름을 빠르게 이해할 수 있도록 간단히 정리했습니다.
