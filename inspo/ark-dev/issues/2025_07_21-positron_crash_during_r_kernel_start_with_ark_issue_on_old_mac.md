# Positron crash during R kernel start with ark issue on old Mac

> <https://github.com/posit-dev/ark/issues/876>
> 
> * Author: @aleahy-work
> * State: OPEN
> * Labels: 

<!--

Thanks for taking the time to file an issue!

While Ark aims to be a multi-IDE project, its main target is the Positron IDE (https://github.com/posit-dev/positron) and we coordinate all bug reports and feature requests relevant for Positron in its Github issues page. Please consider posting your issue at https://github.com/posit-dev/positron/issues.

Examples of issues that should be posted to the Ark repository:

- Internal tasks and refactoring.
- Issues and feature requests that only concern Jupyter applications and that do not affect Positron.

Examples of issues that should be posted to the Positron repository:

- Issues and feature requests to components of Ark that directly affect the experience in Positron, such as LSP support.
- RStudio API support.

-->


This seems related to https://github.com/posit-dev/positron/issues/3812, which was marked as completed on February 12. When I start an R session on my MacOS 11.7 system (which should be supported according to https://positron.posit.co/download.html )  it fails with:

R 4.5.1 failed to start up (exit code -1)

The kernel exited before a connection could be established
dyld: Symbol not found: __ZTTNSt3__118basic_stringstreamIcNS_11char_traitsIcEENS_9allocatorIcEEEE
  Referenced from: /Applications/Positron.app/Contents/Resources/app/extensions/positron-r/resources/ark/ark
  Expected in: /usr/lib/libc++.1.dylib
 in /Applications/Positron.app/Contents/Resources/app/extensions/positron-r/resources/ark/ark

About positron:

Positron Version: 2025.07.0 build 204
Code - OSS Version: 1.100.3
Commit: 03ae7a5393c776bb24c23d2aa6a6bfbba90cbc5e
Date: 2025-06-30T16:28:59.945Z
Electron: 34.5.1
Chromium: 132.0.6834.210
Node.js: 20.19.0
V8: 13.2.152.41-electron.0
OS: Darwin x64 20.6.0


The lengthy version from the 'Problem Report' is below.

Process:               ark [23812]
Path:                  /Applications/Positron.app/Contents/Resources/app/extensions/positron-r/resources/ark/ark
Identifier:            ark
Version:               0
Code Type:             X86-64 (Native)
Parent Process:        kcserver [23597]
Responsible:           Electron [23081]
User ID:               501

Date/Time:             2025-07-17 14:03:10.818 -0500
OS Version:            macOS 11.7.10 (20G1427)
Report Version:        12
Anonymous UUID:        13A59FE3-DC9B-466F-0AC2-BEAAD172C918

Sleep/Wake UUID:       908C7EAC-E1E8-4DBE-985A-87D05BFC0739

Time Awake Since Boot: 20000 seconds
Time Since Wake:       3900 seconds

System Integrity Protection: enabled

Crashed Thread:        0

Exception Type:        EXC_CRASH (SIGABRT)
Exception Codes:       0x0000000000000000, 0x0000000000000000
Exception Note:        EXC_CORPSE_NOTIFY

Termination Reason:    DYLD, [0x4] Symbol missing

Application Specific Information:
dyld: launch, loading dependent libraries
DYLD_LIBRARY_PATH=/Library/Frameworks/R.framework/Resources/lib

Dyld Error Message:
  Symbol not found: __ZTTNSt3__118basic_stringstreamIcNS_11char_traitsIcEENS_9allocatorIcEEEE
  Referenced from: /Applications/Positron.app/Contents/Resources/app/extensions/positron-r/resources/ark/ark
  Expected in: /usr/lib/libc++.1.dylib
 in /Applications/Positron.app/Contents/Resources/app/extensions/positron-r/resources/ark/ark

Binary Images:
       0x100f74000 -        0x101d99fff +ark (0) <45688BB8-E895-3B11-9C0B-64BB7D5BBE23> /Applications/Positron.app/Contents/Resources/app/extensions/positron-r/resources/ark/ark
       0x10907c000 -        0x109117fff  dyld (852.2) <BD607394-9008-33B9-B98B-A5886668E52C> /usr/lib/dyld
    0x7fff202ff000 -     0x7fff20300fff  libsystem_blocks.dylib (79) <F5B25F38-FC21-3BF5-A147-3B913DA098BE> /usr/lib/system/libsystem_blocks.dylib
    0x7fff20301000 -     0x7fff20336fff  libxpc.dylib (2038.120.1.701.2) <151C64CA-CA6F-3989-A558-796EB6ED0C11> /usr/lib/system/libxpc.dylib
    0x7fff20337000 -     0x7fff2034efff  libsystem_trace.dylib (1277.120.1) <1F20357C-395F-3095-B525-AD9403290A92> /usr/lib/system/libsystem_trace.dylib
    0x7fff2034f000 -     0x7fff203ecfff  libcorecrypto.dylib (1000.140.4) <BDD3FF5E-34F8-3AC0-A05C-F9AC17C88BBF> /usr/lib/system/libcorecrypto.dylib
    0x7fff203ed000 -     0x7fff20419fff  libsystem_malloc.dylib (317.140.5) <3AB4C7E9-C49C-3EB7-9370-370F3F655024> /usr/lib/system/libsystem_malloc.dylib
    0x7fff2041a000 -     0x7fff2045efff  libdispatch.dylib (1271.120.2) <5D824C33-C5E2-38A8-BD00-D934443DBDAB> /usr/lib/system/libdispatch.dylib
    0x7fff2045f000 -     0x7fff20498fff  libobjc.A.dylib (824.1) <A0961DED-3477-3856-A6BC-CFE2475CB2F4> /usr/lib/libobjc.A.dylib
    0x7fff20499000 -     0x7fff2049bfff  libsystem_featureflags.dylib (28.60.1) <2BAC8770-AFC8-3FE2-B6C6-27CE44B2B2BA> /usr/lib/system/libsystem_featureflags.dylib
    0x7fff2049c000 -     0x7fff20524fff  libsystem_c.dylib (1439.141.1) <BC8BCEEA-CA52-32C7-9FF5-E444CF9EF66A> /usr/lib/system/libsystem_c.dylib
    0x7fff20525000 -     0x7fff2057afff  libc++.1.dylib (905.6) <5BA6B5ED-7842-3B13-86B0-00EB511CE2FE> /usr/lib/libc++.1.dylib
    0x7fff2057b000 -     0x7fff20590fff  libc++abi.dylib (905.6) <B96FC1DD-0056-3E11-862A-C0BB8239FEA0> /usr/lib/libc++abi.dylib
    0x7fff20591000 -     0x7fff205c0fff  libsystem_kernel.dylib (7195.141.49.702.12) <BA061E84-6D44-3037-832D-E86D783FA917> /usr/lib/system/libsystem_kernel.dylib
    0x7fff205c1000 -     0x7fff205ccfff  libsystem_pthread.dylib (454.120.2.700.1) <409239A7-2E4E-31C7-87EB-EE50B7981204> /usr/lib/system/libsystem_pthread.dylib
    0x7fff205cd000 -     0x7fff20608fff  libdyld.dylib (852.2) <FD8DB5BC-F199-3524-9DC4-DAEC0E94712F> /usr/lib/system/libdyld.dylib
    0x7fff20609000 -     0x7fff20612fff  libsystem_platform.dylib (254.80.2) <52A77346-8AA5-3BB7-906D-C7503B491CF9> /usr/lib/system/libsystem_platform.dylib
    0x7fff20613000 -     0x7fff2063efff  libsystem_info.dylib (542.40.4) <406353B2-E48A-3D20-B08F-0AB26ED8A0B3> /usr/lib/system/libsystem_info.dylib
    0x7fff2063f000 -     0x7fff20adcfff  com.apple.CoreFoundation (6.9 - 1778.105) <B4B8042A-9415-3F26-91AC-735C968B0D95> /System/Library/Frameworks/CoreFoundation.framework/Versions/A/CoreFoundation
    0x7fff20add000 -     0x7fff20d14fff  com.apple.LaunchServices (1122.45 - 1122.45) <42ED2E08-904B-3B62-B0B6-DACBE4988AAB> /System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/LaunchServices
    0x7fff20d15000 -     0x7fff20de9fff  com.apple.gpusw.MetalTools (1.0 - 1) <72285C8A-5F98-31A0-9CA1-30CF4387584B> /System/Library/PrivateFrameworks/MetalTools.framework/Versions/A/MetalTools
    0x7fff20dea000 -     0x7fff21046fff  libBLAS.dylib (1336.140.1) <D4B16233-BAE7-3D63-BB59-5DCEC63345EB> /System/Library/Frameworks/Accelerate.framework/Versions/A/Frameworks/vecLib.framework/Versions/A/libBLAS.dylib
    0x7fff21047000 -     0x7fff21094fff  com.apple.Lexicon-framework (1.0 - 86.2) <09EC8AE4-7FC7-3D2D-A6DD-C484B664B1D5> /System/Library/PrivateFrameworks/Lexicon.framework/Versions/A/Lexicon
    0x7fff21095000 -     0x7fff21103fff  libSparse.dylib (106) <0FD77742-B7DB-3296-9D0F-0DEF7EB4FF7D> /System/Library/Frameworks/Accelerate.framework/Versions/A/Frameworks/vecLib.framework/Versions/A/libSparse.dylib
    0x7fff21104000 -     0x7fff21181fff  com.apple.SystemConfiguration (1.20 - 1.20) <D59BEA1F-BD5D-383A-8977-64F5B72F16C4> /System/Library/Frameworks/SystemConfiguration.framework/Versions/A/SystemConfiguration
    0x7fff21182000 -     0x7fff211b6fff  libCRFSuite.dylib (50) <2DADF4F9-0BD3-33CF-9939-979E69F2453C> /usr/lib/libCRFSuite.dylib
    0x7fff211b7000 -     0x7fff213effff  libmecabra.dylib (929.10) <58AA4922-A668-3165-802C-5FB4DF848E40> /usr/lib/libmecabra.dylib
    0x7fff213f0000 -     0x7fff2174efff  com.apple.Foundation (6.9 - 1778.105) <4F4709DD-C198-3AA1-86A0-71D2F2FDD65D> /System/Library/Frameworks/Foundation.framework/Versions/C/Foundation
    0x7fff2174f000 -     0x7fff21837fff  com.apple.LanguageModeling (1.0 - 247.3) <EAAF99AF-2D5F-3EC5-B7F7-41D7236A09F3> /System/Library/PrivateFrameworks/LanguageModeling.framework/Versions/A/LanguageModeling
    0x7fff22458000 -     0x7fff227affff  com.apple.security (7.0 - 59754.141.1.702.3) <5A52B8E8-B1AF-3F29-AC97-5DBEE8C6A6AC> /System/Library/Frameworks/Security.framework/Versions/A/Security
    0x7fff227b0000 -     0x7fff22a0ffff  libicucore.A.dylib (66112.1) <9F2A881A-25DA-3386-9DCE-D2B67C2A4141> /usr/lib/libicucore.A.dylib
    0x7fff22a10000 -     0x7fff22a19fff  libsystem_darwin.dylib (1439.141.1) <75592BEC-777B-381F-8C07-15B8A4C712A7> /usr/lib/system/libsystem_darwin.dylib
    0x7fff22a1a000 -     0x7fff22d05fff  com.apple.CoreServices.CarbonCore (1307.3 - 1307.3) <76566083-9F9C-3055-812A-079693A69D32> /System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/CarbonCore.framework/Versions/A/CarbonCore
    0x7fff22d45000 -     0x7fff22d7ffff  com.apple.CSStore (1122.45 - 1122.45) <65919E05-BE7E-39AC-8768-B32E41E325C0> /System/Library/PrivateFrameworks/CoreServicesStore.framework/Versions/A/CoreServicesStore
    0x7fff22d80000 -     0x7fff22e2efff  com.apple.framework.IOKit (2.0.2 - 1845.120.6) <A395F442-1253-3CA9-953F-7A235EEB7F67> /System/Library/Frameworks/IOKit.framework/Versions/A/IOKit
    0x7fff22e2f000 -     0x7fff22e3afff  libsystem_notify.dylib (279.40.4) <02E22D9D-01E2-361C-BB9A-B5BE18D28280> /usr/lib/system/libsystem_notify.dylib
    0x7fff242a1000 -     0x7fff24927fff  libnetwork.dylib (2288.140.9) <2DE517EE-E318-366B-A7FA-AD5F62D007CB> /usr/lib/libnetwork.dylib
    0x7fff24928000 -     0x7fff24dc5fff  com.apple.CFNetwork (1240.0.4.5 - 1240.0.4.5) <83B8DEAA-82EE-36DD-ADF8-45E8A807BC21> /System/Library/Frameworks/CFNetwork.framework/Versions/A/CFNetwork
    0x7fff24dc6000 -     0x7fff24dd4fff  libsystem_networkextension.dylib (1295.140.4.701.1) <9C5A85AC-C593-34FD-8481-5CFC05DE3897> /usr/lib/system/libsystem_networkextension.dylib
    0x7fff24dd5000 -     0x7fff24dd5fff  libenergytrace.dylib (22.100.1) <EDE247D7-22AC-3339-AC3E-04A5BD13E3F2> /usr/lib/libenergytrace.dylib
    0x7fff24dd6000 -     0x7fff24e32fff  libMobileGestalt.dylib (978.140.1) <AC0BF1F3-5052-3FD8-808D-CBF55B3F7551> /usr/lib/libMobileGestalt.dylib
    0x7fff24e33000 -     0x7fff24e49fff  libsystem_asl.dylib (385.0.2) <88F4051D-1CF5-314E-A952-247C38996E16> /usr/lib/system/libsystem_asl.dylib
    0x7fff24e4a000 -     0x7fff24e62fff  com.apple.TCC (1.0 - 1) <898C8BE6-EBC0-3BEB-B898-2EF336802530> /System/Library/PrivateFrameworks/TCC.framework/Versions/A/TCC
    0x7fff26183000 -     0x7fff26337fff  libsqlite3.dylib (321.4) <2CBF5CD2-BECF-331B-904C-A88A54C6F6ED> /usr/lib/libsqlite3.dylib
    0x7fff26496000 -     0x7fff2650afff  com.apple.AE (918.6 - 918.6) <677BFC57-B830-3090-9470-A21CB2A77C76> /System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/AE.framework/Versions/A/AE
    0x7fff2650b000 -     0x7fff26511fff  libdns_services.dylib (1310.140.1) <EABE9A6A-96DE-3A2E-B0E0-17F277A65757> /usr/lib/libdns_services.dylib
    0x7fff26512000 -     0x7fff26519fff  libsystem_symptoms.dylib (1431.140.1) <E9CB193F-260B-3835-B76E-A2209343FA1E> /usr/lib/system/libsystem_symptoms.dylib
    0x7fff266a6000 -     0x7fff266d5fff  com.apple.analyticsd (1.0 - 1) <23CB7B45-967B-37B3-AF21-21B4885790CC> /System/Library/PrivateFrameworks/CoreAnalytics.framework/Versions/A/CoreAnalytics
    0x7fff266d6000 -     0x7fff266d8fff  libDiagnosticMessagesClient.dylib (112) <8CE0D64A-597F-3048-80C3-590D866D067A> /usr/lib/libDiagnosticMessagesClient.dylib
    0x7fff266d9000 -     0x7fff26725fff  com.apple.spotlight.metadata.utilities (1.0 - 2150.30) <9B61E5D5-27C3-3282-A650-A2D15FA76FF7> /System/Library/PrivateFrameworks/MetadataUtilities.framework/Versions/A/MetadataUtilities
    0x7fff26726000 -     0x7fff267c0fff  com.apple.Metadata (10.7.0 - 2150.30) <FEBC2256-7D84-3F2E-A770-A8665F62E20A> /System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/Metadata.framework/Versions/A/Metadata
    0x7fff267c1000 -     0x7fff267c7fff  com.apple.DiskArbitration (2.7 - 2.7) <21325211-A5F7-3AB9-BDFE-6B6DC06E587E> /System/Library/Frameworks/DiskArbitration.framework/Versions/A/DiskArbitration
    0x7fff267c8000 -     0x7fff26e2ffff  com.apple.vImage (8.1 - 544.6) <1DD123D7-ACC3-3FCB-838E-C91C6E4D31B8> /System/Library/Frameworks/Accelerate.framework/Versions/A/Frameworks/vImage.framework/Versions/A/vImage
    0x7fff27389000 -     0x7fff27398fff  com.apple.OpenDirectory (11.7 - 230.40.1) <B7BB547E-B00F-37B3-A4A8-AF414F029E64> /System/Library/Frameworks/OpenDirectory.framework/Versions/A/OpenDirectory
    0x7fff27399000 -     0x7fff273b8fff  com.apple.CFOpenDirectory (11.7 - 230.40.1) <E4682D99-DD7C-3C74-A0A1-E561B6E616C6> /System/Library/Frameworks/OpenDirectory.framework/Versions/A/Frameworks/CFOpenDirectory.framework/Versions/A/CFOpenDirectory
    0x7fff273b9000 -     0x7fff273c5fff  com.apple.CoreServices.FSEvents (1290.120.6 - 1290.120.6) <78184C84-4633-3867-AACD-8F0256F40D5A> /System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/FSEvents.framework/Versions/A/FSEvents
    0x7fff273c6000 -     0x7fff273eafff  com.apple.coreservices.SharedFileList (144 - 144) <243CAB7D-EA1A-3322-9833-B4B24F63AB3E> /System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/SharedFileList.framework/Versions/A/SharedFileList
    0x7fff273eb000 -     0x7fff273edfff  libapp_launch_measurement.dylib (14.1) <2AE731D8-757E-3A23-8375-9D266B762CC3> /usr/lib/libapp_launch_measurement.dylib
    0x7fff273ee000 -     0x7fff27435fff  com.apple.CoreAutoLayout (1.0 - 21.10.1) <32846C89-8FED-3225-B370-34FB1DA82A85> /System/Library/PrivateFrameworks/CoreAutoLayout.framework/Versions/A/CoreAutoLayout
    0x7fff27436000 -     0x7fff27518fff  libxml2.2.dylib (34.26) <29CE75F5-D4D3-35BD-9B89-3B8970980C55> /usr/lib/libxml2.2.dylib
    0x7fff28521000 -     0x7fff28531fff  libsystem_containermanager.dylib (318.100.4.700.1) <45445167-AFC7-3406-A858-9AE8D8F45907> /usr/lib/system/libsystem_containermanager.dylib
    0x7fff28532000 -     0x7fff28543fff  com.apple.IOSurface (290.8.2 - 290.8.2) <B98B7126-FFF8-343F-BE66-32212DE3BEBE> /System/Library/Frameworks/IOSurface.framework/Versions/A/IOSurface
    0x7fff28544000 -     0x7fff2854dfff  com.apple.IOAccelerator (442.10 - 442.10) <19FBA808-F918-3BB2-BE78-A1B0D10D724D> /System/Library/PrivateFrameworks/IOAccelerator.framework/Versions/A/IOAccelerator
    0x7fff2854e000 -     0x7fff28671fff  com.apple.Metal (244.303 - 244.303) <A9397F90-E221-397B-BA10-B52135A72D68> /System/Library/Frameworks/Metal.framework/Versions/A/Metal
    0x7fff291cf000 -     0x7fff29235fff  com.apple.MetalPerformanceShaders.MPSCore (1.0 - 1) <02F2E0C6-0C0F-3390-A63B-189832967015> /System/Library/Frameworks/MetalPerformanceShaders.framework/Versions/A/Frameworks/MPSCore.framework/Versions/A/MPSCore
    0x7fff29236000 -     0x7fff29239fff  libsystem_configuration.dylib (1109.140.1) <53B71513-3009-3A8C-A5AA-9C15DD0AB54E> /usr/lib/system/libsystem_configuration.dylib
    0x7fff2923a000 -     0x7fff2923efff  libsystem_sandbox.dylib (1441.141.13.701.2) <1E19BC49-484C-32BB-8BB7-99D41C63F86E> /usr/lib/system/libsystem_sandbox.dylib
    0x7fff2923f000 -     0x7fff29240fff  com.apple.AggregateDictionary (1.0 - 1) <CD5E6E8F-7AB6-345E-9243-D5D674DC0225> /System/Library/PrivateFrameworks/AggregateDictionary.framework/Versions/A/AggregateDictionary
    0x7fff29241000 -     0x7fff29244fff  com.apple.AppleSystemInfo (3.1.5 - 3.1.5) <15CBB967-FAAE-3A22-A87F-4833A9D835E3> /System/Library/PrivateFrameworks/AppleSystemInfo.framework/Versions/A/AppleSystemInfo
    0x7fff29245000 -     0x7fff29246fff  liblangid.dylib (136) <D6DDBEB6-7A9A-3F00-8DEF-18934CFC0A08> /usr/lib/liblangid.dylib
    0x7fff29247000 -     0x7fff292ebfff  com.apple.CoreNLP (1.0 - 245.2) <F40C2289-9A6D-3C55-A6DA-FFAD41636415> /System/Library/PrivateFrameworks/CoreNLP.framework/Versions/A/CoreNLP
    0x7fff292ec000 -     0x7fff292f2fff  com.apple.LinguisticData (1.0 - 399) <E6DC793D-3133-3D9B-BCF8-E4A628E45586> /System/Library/PrivateFrameworks/LinguisticData.framework/Versions/A/LinguisticData
    0x7fff292f3000 -     0x7fff2999bfff  libBNNS.dylib (288.100.5) <1E45AC70-6C75-3F27-9252-40DF6B2D674A> /System/Library/Frameworks/Accelerate.framework/Versions/A/Frameworks/vecLib.framework/Versions/A/libBNNS.dylib
    0x7fff2999c000 -     0x7fff29b6efff  libvDSP.dylib (760.100.3) <7F1276C0-C9F6-3C6F-A0F7-1EB4EA666BD8> /System/Library/Frameworks/Accelerate.framework/Versions/A/Frameworks/vecLib.framework/Versions/A/libvDSP.dylib
    0x7fff29b6f000 -     0x7fff29b80fff  com.apple.CoreEmoji (1.0 - 128.4) <011AA15B-6988-3F36-81A3-2B52B561D6E0> /System/Library/PrivateFrameworks/CoreEmoji.framework/Versions/A/CoreEmoji
    0x7fff29b81000 -     0x7fff29b8bfff  com.apple.IOMobileFramebuffer (343.0.0 - 343.0.0) <28991DA2-1726-3F77-A9C5-4BB5AAEFA166> /System/Library/PrivateFrameworks/IOMobileFramebuffer.framework/Versions/A/IOMobileFramebuffer
    0x7fff29e95000 -     0x7fff29f20fff  com.apple.securityfoundation (6.0 - 55240.40.4) <D1E23625-27EF-37F4-93B8-E3162C1943BA> /System/Library/Frameworks/SecurityFoundation.framework/Versions/A/SecurityFoundation
    0x7fff29f21000 -     0x7fff29f2afff  com.apple.coreservices.BackgroundTaskManagement (1.0 - 104) <8CF5B495-3026-3CE1-9EFC-8D7D71380A43> /System/Library/PrivateFrameworks/BackgroundTaskManagement.framework/Versions/A/BackgroundTaskManagement
    0x7fff29f2b000 -     0x7fff29f2ffff  com.apple.xpc.ServiceManagement (1.0 - 1) <D561E8B7-690C-3D18-A1E8-C4B01B8B9C11> /System/Library/Frameworks/ServiceManagement.framework/Versions/A/ServiceManagement
    0x7fff29f30000 -     0x7fff29f32fff  libquarantine.dylib (119.40.4) <21C63859-6DFB-3463-9ADF-BB44FB28067C> /usr/lib/system/libquarantine.dylib
    0x7fff29f33000 -     0x7fff29f3efff  libCheckFix.dylib (31) <1C2B822D-29D6-36E2-BBA3-F72DE49E038B> /usr/lib/libCheckFix.dylib
    0x7fff29f3f000 -     0x7fff29f56fff  libcoretls.dylib (169.100.1) <FC8265A0-9659-35D9-BA6F-6507A44742FE> /usr/lib/libcoretls.dylib
    0x7fff29f57000 -     0x7fff29f67fff  libbsm.0.dylib (68.40.1) <0CF67F8A-268D-320A-A3A4-D7C2D9AB8027> /usr/lib/libbsm.0.dylib
    0x7fff29f68000 -     0x7fff29fb1fff  libmecab.dylib (929.10) <47A982DF-1436-366E-AC45-1DA068832AED> /usr/lib/libmecab.dylib
    0x7fff29fb2000 -     0x7fff29fb7fff  libgermantok.dylib (24) <189F508A-723B-345D-918F-178CF15077F3> /usr/lib/libgermantok.dylib
    0x7fff29fb8000 -     0x7fff29fcdfff  libLinearAlgebra.dylib (1336.140.1) <27358E5F-256F-309F-AAC8-BAC4A56C7BF4> /System/Library/Frameworks/Accelerate.framework/Versions/A/Frameworks/vecLib.framework/Versions/A/libLinearAlgebra.dylib
    0x7fff29fce000 -     0x7fff2a1ecfff  com.apple.MetalPerformanceShaders.MPSNeuralNetwork (1.0 - 1) <B7F8218A-2DA2-35A4-9200-3BD52CCF125C> /System/Library/Frameworks/MetalPerformanceShaders.framework/Versions/A/Frameworks/MPSNeuralNetwork.framework/Versions/A/MPSNeuralNetwork
    0x7fff2a1ed000 -     0x7fff2a23cfff  com.apple.MetalPerformanceShaders.MPSRayIntersector (1.0 - 1) <3993AC67-62B4-3E49-B5BF-E8F814CE6C97> /System/Library/Frameworks/MetalPerformanceShaders.framework/Versions/A/Frameworks/MPSRayIntersector.framework/Versions/A/MPSRayIntersector
    0x7fff2a23d000 -     0x7fff2a39efff  com.apple.MLCompute (1.0 - 1) <6026D664-0453-321F-81FE-A40AD902849E> /System/Library/Frameworks/MLCompute.framework/Versions/A/MLCompute
    0x7fff2a39f000 -     0x7fff2a3d5fff  com.apple.MetalPerformanceShaders.MPSMatrix (1.0 - 1) <A194A321-8DD9-3051-97EC-3C4630946007> /System/Library/Frameworks/MetalPerformanceShaders.framework/Versions/A/Frameworks/MPSMatrix.framework/Versions/A/MPSMatrix
    0x7fff2a3d6000 -     0x7fff2a42cfff  com.apple.MetalPerformanceShaders.MPSNDArray (1.0 - 1) <A72429D4-3BED-34DD-BEDE-322A0975A8BC> /System/Library/Frameworks/MetalPerformanceShaders.framework/Versions/A/Frameworks/MPSNDArray.framework/Versions/A/MPSNDArray
    0x7fff2a42d000 -     0x7fff2a4bdfff  com.apple.MetalPerformanceShaders.MPSImage (1.0 - 1) <0B333F06-FAD5-3689-9017-15334AD4F51C> /System/Library/Frameworks/MetalPerformanceShaders.framework/Versions/A/Frameworks/MPSImage.framework/Versions/A/MPSImage
    0x7fff2a4be000 -     0x7fff2a4cdfff  com.apple.AppleFSCompression (125 - 1.0) <1C5279EE-8F78-386E-9E4D-24A3785CACA2> /System/Library/PrivateFrameworks/AppleFSCompression.framework/Versions/A/AppleFSCompression
    0x7fff2a4ce000 -     0x7fff2a4dafff  libbz2.1.0.dylib (44) <6E82D414-3810-36CF-94FF-B1BDF48DB501> /usr/lib/libbz2.1.0.dylib
    0x7fff2a4db000 -     0x7fff2a4dffff  libsystem_coreservices.dylib (127.1) <6D84FA08-CB2B-34E1-9AB4-A54E82CB9161> /usr/lib/system/libsystem_coreservices.dylib
    0x7fff2a4e0000 -     0x7fff2a50dfff  com.apple.CoreServices.OSServices (1122.45 - 1122.45) <097586DB-22C5-323A-BC5C-5AF75613846D> /System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/OSServices.framework/Versions/A/OSServices
    0x7fff2a6dc000 -     0x7fff2a6eefff  libz.1.dylib (76.140.1) <A2FF8D14-1632-3047-9829-CC90239F97FF> /usr/lib/libz.1.dylib
    0x7fff2a6ef000 -     0x7fff2a736fff  libsystem_m.dylib (3186.100.3) <1836B380-C579-3195-BC3F-77404D432186> /usr/lib/system/libsystem_m.dylib
    0x7fff2a737000 -     0x7fff2a737fff  libcharset.1.dylib (59) <3A46C22D-E678-356B-9BAD-6E837704D662> /usr/lib/libcharset.1.dylib
    0x7fff2a738000 -     0x7fff2a73dfff  libmacho.dylib (980) <F7BDAFE5-4E49-39DD-8F94-CD5E49C91A90> /usr/lib/system/libmacho.dylib
    0x7fff2a73e000 -     0x7fff2a759fff  libkxld.dylib (7195.141.49.702.12) <6585C769-FACC-3E47-844B-C7011292F3C5> /usr/lib/system/libkxld.dylib
    0x7fff2a75a000 -     0x7fff2a765fff  libcommonCrypto.dylib (60178.120.3) <B057F752-3057-394D-A3F6-AA11A04A6392> /usr/lib/system/libcommonCrypto.dylib
    0x7fff2a766000 -     0x7fff2a770fff  libunwind.dylib (201) <9D6A6228-8DC3-3521-B458-4EDE4A9F5E65> /usr/lib/system/libunwind.dylib
    0x7fff2a771000 -     0x7fff2a778fff  liboah.dylib (203.58) <AC9E8A76-FCAA-3F97-802A-D22EF770463B> /usr/lib/liboah.dylib
    0x7fff2a779000 -     0x7fff2a783fff  libcopyfile.dylib (173.40.2) <BD7EAE7B-28C1-36DF-96B8-F506D50DFF28> /usr/lib/system/libcopyfile.dylib
    0x7fff2a784000 -     0x7fff2a78bfff  libcompiler_rt.dylib (102.2) <BA910DC2-C697-3DAD-9A70-7C8CD5217AC3> /usr/lib/system/libcompiler_rt.dylib
    0x7fff2a78c000 -     0x7fff2a78efff  libsystem_collections.dylib (1439.141.1) <21F2EF42-56ED-3E0F-9C29-94E0888DC52C> /usr/lib/system/libsystem_collections.dylib
    0x7fff2a78f000 -     0x7fff2a791fff  libsystem_secinit.dylib (87.60.1) <E976428F-F9E2-334B-AA91-9AAD40234718> /usr/lib/system/libsystem_secinit.dylib
    0x7fff2a792000 -     0x7fff2a794fff  libremovefile.dylib (49.120.1) <5AC9F8EC-F0E8-3D8A-ADB5-96B5FB581896> /usr/lib/system/libremovefile.dylib
    0x7fff2a795000 -     0x7fff2a795fff  libkeymgr.dylib (31) <9FBE08F6-0679-3976-AFDC-1EAF40C3958F> /usr/lib/system/libkeymgr.dylib
    0x7fff2a796000 -     0x7fff2a79dfff  libsystem_dnssd.dylib (1310.140.1) <8C4D6C93-285F-3587-A986-5BB96A1C664F> /usr/lib/system/libsystem_dnssd.dylib
    0x7fff2a79e000 -     0x7fff2a7a3fff  libcache.dylib (83) <56DCEFF5-111E-32FD-B4E9-E148507C4FEC> /usr/lib/system/libcache.dylib
    0x7fff2a7a4000 -     0x7fff2a7a5fff  libSystem.B.dylib (1292.120.1) <A8E7368E-58FA-31E5-8D4D-FC2FED6100E6> /usr/lib/libSystem.B.dylib
    0x7fff2a7a6000 -     0x7fff2a7a9fff  libfakelink.dylib (3) <6002BC93-3627-366E-8D21-A552D56CB215> /usr/lib/libfakelink.dylib
    0x7fff2a7aa000 -     0x7fff2a7aafff  com.apple.SoftLinking (1.0 - 1) <3D0CEDFD-B263-39CA-8B31-E0A498D05EB3> /System/Library/PrivateFrameworks/SoftLinking.framework/Versions/A/SoftLinking
    0x7fff2a7ab000 -     0x7fff2a7e2fff  libpcap.A.dylib (98.100.3) <236EE73F-6D38-38E0-9BC0-B427DEB7F9FD> /usr/lib/libpcap.A.dylib
    0x7fff2a7e3000 -     0x7fff2a8d3fff  libiconv.2.dylib (59) <DEE0153A-BDF9-33CA-B8C7-3C39DB906B5E> /usr/lib/libiconv.2.dylib
    0x7fff2a8d4000 -     0x7fff2a8e5fff  libcmph.dylib (8) <83A69507-07D1-387F-9D06-1011E7909EAC> /usr/lib/libcmph.dylib
    0x7fff2a8e6000 -     0x7fff2a957fff  libarchive.2.dylib (83.100.2) <45B577F5-0064-3E73-89B8-BE4A121B214F> /usr/lib/libarchive.2.dylib
    0x7fff2a958000 -     0x7fff2a9bffff  com.apple.SearchKit (1.4.1 - 1.4.1) <7C264603-379D-38BF-A3EC-49C01059C5E5> /System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/SearchKit.framework/Versions/A/SearchKit
    0x7fff2a9c0000 -     0x7fff2a9c1fff  libThaiTokenizer.dylib (3) <BA265C01-176E-3F7D-97F6-7FAABB0CAEC8> /usr/lib/libThaiTokenizer.dylib
    0x7fff2a9c2000 -     0x7fff2a9e4fff  com.apple.applesauce (1.0 - 16.28) <EAFF4FEC-51F3-3D0D-9D99-E62E75937F1B> /System/Library/PrivateFrameworks/AppleSauce.framework/Versions/A/AppleSauce
    0x7fff2a9e5000 -     0x7fff2a9fcfff  libapple_nghttp2.dylib (1.41) <AC9520D7-D54F-3031-9503-FEA5A5ED5E56> /usr/lib/libapple_nghttp2.dylib
    0x7fff2a9fd000 -     0x7fff2aa13fff  libSparseBLAS.dylib (1336.140.1) <7D926256-F187-33CA-87D6-74F1660C438A> /System/Library/Frameworks/Accelerate.framework/Versions/A/Frameworks/vecLib.framework/Versions/A/libSparseBLAS.dylib
    0x7fff2aa14000 -     0x7fff2aa15fff  com.apple.MetalPerformanceShaders.MetalPerformanceShaders (1.0 - 1) <9BFE310E-E910-3228-BDF5-21A7C4468D89> /System/Library/Frameworks/MetalPerformanceShaders.framework/Versions/A/MetalPerformanceShaders
    0x7fff2aa16000 -     0x7fff2aa1bfff  libpam.2.dylib (28.40.1.700.1) <564320AF-69E5-3FEE-BE3A-E500B9B6786F> /usr/lib/libpam.2.dylib
    0x7fff2aa1c000 -     0x7fff2aa3bfff  libcompression.dylib (96.120.1) <F36054C1-6074-3A22-82EF-6F4A2A52599C> /usr/lib/libcompression.dylib
    0x7fff2aa3c000 -     0x7fff2aa41fff  libQuadrature.dylib (7) <256CB21E-2878-3F22-B4B5-E1FB60D64C9E> /System/Library/Frameworks/Accelerate.framework/Versions/A/Frameworks/vecLib.framework/Versions/A/libQuadrature.dylib
    0x7fff2aa42000 -     0x7fff2addffff  libLAPACK.dylib (1336.140.1) <02F2D4D1-8763-32D1-B5F9-9DD439EFC8E8> /System/Library/Frameworks/Accelerate.framework/Versions/A/Frameworks/vecLib.framework/Versions/A/libLAPACK.dylib
    0x7fff2ade0000 -     0x7fff2ae2ffff  com.apple.DictionaryServices (1.2 - 341) <FB843860-C7D5-3060-B50E-303A3CBAE9A9> /System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/DictionaryServices.framework/Versions/A/DictionaryServices
    0x7fff2ae30000 -     0x7fff2ae48fff  liblzma.5.dylib (16) <455C9083-014D-3037-AC54-1395F3796734> /usr/lib/liblzma.5.dylib
    0x7fff2ae49000 -     0x7fff2ae4afff  libcoretls_cfhelpers.dylib (169.100.1) <6760D250-2628-3DA2-A8A4-6F438E09527A> /usr/lib/libcoretls_cfhelpers.dylib
    0x7fff2ae4b000 -     0x7fff2af46fff  com.apple.APFS (1677.141.3 - 1677.141.3) <E4B0DF0F-E1A5-3FEF-A2A6-8105AD54D95A> /System/Library/PrivateFrameworks/APFS.framework/Versions/A/APFS
    0x7fff2af47000 -     0x7fff2af55fff  libxar.1.dylib (452.140.1) <9E460111-1BBC-31FE-8CAF-FA8AEC22C1E9> /usr/lib/libxar.1.dylib
    0x7fff2af56000 -     0x7fff2af59fff  libutil.dylib (58.40.3) <B5961283-0856-3D78-AE9C-EAFB6A903569> /usr/lib/libutil.dylib
    0x7fff2af5a000 -     0x7fff2af82fff  libxslt.1.dylib (17.10) <52B300FD-B3F6-3689-9554-98B543A298C7> /usr/lib/libxslt.1.dylib
    0x7fff2af83000 -     0x7fff2af8dfff  libChineseTokenizer.dylib (37.1) <62BC78D3-1400-3366-A04E-C8BEE6AC00B5> /usr/lib/libChineseTokenizer.dylib
    0x7fff2af8e000 -     0x7fff2b04bfff  libvMisc.dylib (760.100.3) <560739C2-D16B-36CA-89F4-BD4DD2192333> /System/Library/Frameworks/Accelerate.framework/Versions/A/Frameworks/vecLib.framework/Versions/A/libvMisc.dylib
    0x7fff2dbac000 -     0x7fff2dbacfff  liblaunch.dylib (2038.120.1.701.2) <B79B00B1-954F-3EC4-9E22-A24E25CAE88D> /usr/lib/system/liblaunch.dylib
    0x7fff30048000 -     0x7fff30048fff  libsystem_product_info_filter.dylib (8.40.1) <BB06C92C-6BD7-310C-A176-DC0DCE8D9F2B> /usr/lib/system/libsystem_product_info_filter.dylib
    0x7fff30120000 -     0x7fff30120fff  com.apple.Accelerate.vecLib (3.11 - vecLib 3.11) <F46E0ACF-7524-3CA3-A64A-5DDF6081EB67> /System/Library/Frameworks/Accelerate.framework/Versions/A/Frameworks/vecLib.framework/Versions/A/vecLib
    0x7fff30146000 -     0x7fff30146fff  com.apple.CoreServices (1122.45 - 1122.45) <05DA2462-9BFC-38D9-820A-8842710471D6> /System/Library/Frameworks/CoreServices.framework/Versions/A/CoreServices
    0x7fff30302000 -     0x7fff30302fff  com.apple.Accelerate (1.11 - Accelerate 1.11) <3D8DECC6-19B3-3A32-98CF-EB07536D1635> /System/Library/Frameworks/Accelerate.framework/Versions/A/Accelerate
    0x7fff6bad3000 -     0x7fff6bad9fff  libCoreFSCache.dylib (200.12) <B6360761-3B05-35AE-8E0C-F819414FD093> /System/Library/Frameworks/OpenGL.framework/Versions/A/Libraries/libCoreFSCache.dylib

Model: MacBookAir6,1, BootROM 478.0.0.0.0, 2 processors, Dual-Core Intel Core i5, 1.4 GHz, 4 GB, SMC 2.12f143
Graphics: kHW_IntelHD5000Item, Intel HD Graphics 5000, spdisplays_builtin
Memory Module: BANK 0/DIMM0, 2 GB, DDR3, 1600 MHz, 0x80AD, 0x483943434E4E4E384A544D4C41522D4E544D
Memory Module: BANK 1/DIMM0, 2 GB, DDR3, 1600 MHz, 0x80AD, 0x483943434E4E4E384A544D4C41522D4E544D
AirPort: spairport_wireless_card_type_airport_extreme (0x14E4, 0x117), Broadcom BCM43xx 1.0 (7.77.111.1 AirPortDriverBrcmNIC-1680.11)
Bluetooth: Version 8.0.5d7, 3 services, 19 devices, 1 incoming serial ports
Network Service: Wi-Fi, AirPort, en0
Serial ATA Device: APPLE SSD TS0128F, 121.33 GB
USB Device: USB 3.0 Bus
USB Device: BRCM20702 Hub
USB Device: Bluetooth USB Host Controller
Thunderbolt Bus: MacBook Air, Apple Inc., 23.6


## @kevinushey at 2025-07-21T18:18:51Z

I think this comes down to whether macOS Big Sur can be supported by Positron. This is the newest-available operating system for 2013 / 2014 MacBooks.

Based on the discussion in https://github.com/posit-dev/positron/issues/3812, I suspect this is not feasible; Monterey is likely to be the minimum-required OS version.

## @juliasilge at 2025-07-21T18:51:17Z

I opened https://github.com/posit-dev/positron/issues/8606 for us to update this.

I am sorry for this bad news, @aleahy-work; I am sure it's not what you were hoping to hear. 😔